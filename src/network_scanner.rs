// network_scanner.rs
use std::fs;
use std::io::Write;
use std::net::IpAddr;
use std::time::Duration;
use tokio::time::timeout;
use reqwest::Client;
use local_ip_address::local_ip;
use std::path::Path;
use pnet::datalink;

pub struct NetworkScanner {
    client: Client,
    genesis_port: u16,
    config_path: String,
    interface_name: String,
}

impl NetworkScanner {
    pub fn new(genesis_port: u16, config_path: String, interface_name: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap();
        
        NetworkScanner {
            client,
            genesis_port,
            config_path,
            interface_name,
        }
    }

    pub fn get_interface_ip(interface_name: &str) -> Option<IpAddr> {
        let interfaces = datalink::interfaces();
        
        let interface = interfaces.into_iter()
            .find(|iface| iface.name == interface_name)?;
        
        interface.ips.into_iter()
            .find(|ip| ip.is_ipv4())
            .map(|ip| ip.ip())
    }

    pub fn list_network_interfaces() {
        println!("Available network interfaces:");
        for iface in datalink::interfaces() {
            println!("Name: {}", iface.name);
            println!("  IPs: {:?}", iface.ips);
            println!("  MAC: {:?}", iface.mac);
            println!("  Flags: {}", if iface.is_up() { "UP" } else { "DOWN" });
            println!();
        }
    }

    pub async fn scan_network(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Get IP from specified interface
        let my_ip = Self::get_interface_ip(&self.interface_name)
            .ok_or("Failed to get IP from specified interface")?;

        let network_prefix = match my_ip {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                format!("{}.{}.{}", octets[0], octets[1], octets[2])
            },
            _ => return Err("Only IPv4 is supported".into()),
        };

        println!("Using interface: {} with IP: {}", self.interface_name, my_ip);
        println!("Scanning network: {}.1-254", network_prefix);

        for i in 1..255 {
            let target_ip = format!("{}.{}", network_prefix, i);
            if target_ip == my_ip.to_string() {
                continue; // Skip own IP
            }

            if let Ok(Some(genesis_ip)) = self.check_node(&target_ip).await {
                return Ok(genesis_ip);
            }
        }

        // If no existing genesis node found, become genesis node
        self.become_genesis_node(my_ip.to_string()).await?;
        Ok(my_ip.to_string())
    }

    async fn check_node(&self, ip: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let url = format!("http://{}:{}/isalive", ip, self.genesis_port);
        
        match timeout(Duration::from_secs(1), self.client.get(&url).send()).await {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    if let Ok(genesis_ip) = response.text().await {
                        return Ok(Some(genesis_ip));
                    }
                }
            },
            _ => {} // Timeout or error, continue scanning
        }
        
        Ok(None)
    }

    async fn become_genesis_node(&self, ip: String) -> Result<(), Box<dyn std::error::Error>> {
        println!("No existing genesis node found. Becoming genesis node: {}", ip);
        
        // Create config directory if it doesn't exist
        if let Some(parent) = Path::new(&self.config_path).parent() {
            fs::create_dir_all(parent)?;
        }

        // Write IP to genesis_config.txt
        let mut file = fs::File::create(&self.config_path)?;
        file.write_all(ip.as_bytes())?;
        
        Ok(())
    }
}

// Function to read genesis IP from config
pub fn read_genesis_config(config_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(config_path)?.trim().to_string())
}
