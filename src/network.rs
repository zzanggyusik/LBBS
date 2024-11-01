// network.rs
use std::net::{IpAddr, Ipv4Addr};
use local_ip_address::local_ip;
use reqwest::Client;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use openssl::rsa::{Rsa, Padding};
use openssl::sign::{Signer, Verifier};
use openssl::hash::MessageDigest;
use std::fs::File;
use std::io::Read;
use std::io::{self, Write};
use pnet::datalink;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockData {
    pub ip: String,
    pub command: String,
    pub sign: Vec<u8>
}

pub async fn get_network_info() -> (String, String) {
    // Get all network interfaces
    let interfaces = datalink::interfaces();
    
    // First try to find enp0s8 interface
    let ip_network = interfaces.iter()
        .find(|iface| iface.name == "enp0s8")
        .and_then(|iface| {
            iface.ips.iter()
                .find(|ip| ip.is_ipv4())
                .map(|ip| (ip.ip().to_string(), ip.network()))
        })
        .or_else(|| {
            // If enp0s8 not found, fall back to any other interface with IPv4
            interfaces.iter()
                .filter(|iface| !iface.is_loopback())  // Exclude loopback
                .find_map(|iface| {
                    iface.ips.iter()
                        .find(|ip| ip.is_ipv4())
                        .map(|ip| (ip.ip().to_string(), ip.network()))
                })
        })
        .unwrap_or_else(|| panic!("No suitable network interface found"));

    let my_local_ip = ip_network.0;
    let parts: Vec<&str> = my_local_ip.split('.').collect();
    let network_prefix = format!("{}.{}.{}", parts[0], parts[1], parts[2]);

    // println!("Using network interface with IP: {}", my_local_ip);
    
    (my_local_ip, network_prefix)
}

pub async fn check_node_exists(ip: &str, port: &str) -> bool {
    let client = Client::new();
    let url = format!("http://{}:{}/isalive", ip, port);
    
    match timeout(Duration::from_millis(100), client.get(&url).send()).await {
        Ok(Ok(response)) => response.status().is_success(),
        _ => false
    }
}

pub async fn scan_network() -> Option<String> {
    let (my_ip, network_prefix) = get_network_info().await;
    println!("Scanning network range: {}.1 ~ {}.244", network_prefix, network_prefix);
    
    for i in 1..=244 {
        let target_ip = format!("{}.{}", network_prefix, i);
        if target_ip != my_ip {
            //print!("\rScanning IP: {} ", target_ip); // Progress indicator
            io::stdout().flush().unwrap(); // io:: 접두사 추가
            
            if check_node_exists(&target_ip, crate::config::PORT).await {
                println!("\nFound active node at: {}", target_ip);
                // Found existing node
                let client = Client::new();
                let url = format!("http://{}:{}/genesis-info", target_ip, crate::config::PORT);
                if let Ok(response) = client.get(&url).send().await {
                    if let Ok(genesis_ip) = response.text().await {
                        println!("Retrieved genesis node IP: {}", genesis_ip);
                        return Some(genesis_ip);
                    }
                }
            }
        }
    }
    println!("\nNetwork scan complete. No existing nodes found.");
    None
}

pub fn sign_command(command: &str) -> Vec<u8> {
    let mut key_file = File::open("./key/ca.pem").expect("Failed to open private key");
    let mut key_contents = Vec::new();
    key_file.read_to_end(&mut key_contents).expect("Failed to read private key");
    
    let private_key = Rsa::private_key_from_pem(&key_contents).expect("Failed to parse private key");
    let pkey = openssl::pkey::PKey::from_rsa(private_key).unwrap();
    let mut signer = Signer::new(MessageDigest::sha256(), &pkey).unwrap();
    signer.update(command.as_bytes()).unwrap();
    signer.sign_to_vec().unwrap()
}

pub fn verify_signature(command: &str, signature: &[u8]) -> bool {
    let mut key_file = File::open("./key/ca_public.pem").expect("Failed to open public key");
    let mut key_contents = Vec::new();
    key_file.read_to_end(&mut key_contents).expect("Failed to read public key");
    
    let public_key = Rsa::public_key_from_pem(&key_contents).expect("Failed to parse public key");
    let pkey = openssl::pkey::PKey::from_rsa(public_key).unwrap();
    let mut verifier = Verifier::new(MessageDigest::sha256(), &pkey).unwrap();
    verifier.update(command.as_bytes()).unwrap();
    verifier.verify(signature).unwrap_or(false)
}
