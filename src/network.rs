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

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockData {
    pub ip: String,
    pub command: String,
    pub sign: Vec<u8>
}

pub async fn get_network_info() -> (String, String) {
    let my_local_ip = local_ip().unwrap().to_string();
    let network_prefix = my_local_ip.rpartition('.').0.to_string();
    (my_local_ip, network_prefix)
}

pub async fn check_node_exists(ip: &str, port: &str) -> bool {
    let client = Client::new();
    let url = format!("http://{}:{}/isalive", ip, port);
    
    match timeout(Duration::from_secs(1), client.get(&url).send()).await {
        Ok(Ok(response)) => response.status().is_success(),
        _ => false
    }
}

pub async fn scan_network() -> Option<String> {
    let (my_ip, network_prefix) = get_network_info().await;
    
    for i in 1..=244 {
        let target_ip = format!("{}.{}", network_prefix, i);
        if target_ip != my_ip {
            if check_node_exists(&target_ip, crate::config::PORT).await {
                // Found existing node
                let client = Client::new();
                let url = format!("http://{}:{}/genesis-info", target_ip, crate::config::PORT);
                if let Ok(response) = client.get(&url).send().await {
                    if let Ok(genesis_ip) = response.text().await {
                        return Some(genesis_ip);
                    }
                }
            }
        }
    }
    None
}

pub fn sign_command(command: &str) -> Vec<u8> {
    let mut key_file = File::open("./key/ca.pem").expect("Failed to open private key");
    let mut key_contents = Vec::new();
    key_file.read_to_end(&mut key_contents).expect("Failed to read private key");
    
    let rsa = Rsa::private_key_from_pem(&key_contents).expect("Failed to parse private key");
    let mut signer = Signer::new(MessageDigest::sha256(), &rsa).unwrap();
    signer.update(command.as_bytes()).unwrap();
    signer.sign_to_vec().unwrap()
}

pub fn verify_signature(command: &str, signature: &[u8]) -> bool {
    let mut key_file = File::open("./key/ca_public.pem").expect("Failed to open public key");
    let mut key_contents = Vec::new();
    key_file.read_to_end(&mut key_contents).expect("Failed to read public key");
    
    let rsa = Rsa::public_key_from_pem(&key_contents).expect("Failed to parse public key");
    let mut verifier = Verifier::new(MessageDigest::sha256(), &rsa).unwrap();
    verifier.update(command.as_bytes()).unwrap();
    verifier.verify(signature).unwrap_or(false)
}