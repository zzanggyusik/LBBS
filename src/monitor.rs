use tokio::time::{interval, Duration};
use crate::network::{get_network_info, scan_network};
use crate::config::{self, PORT, save_genesis_config, save_node_list};
use reqwest::Client;

pub struct NetworkMonitor {
    last_network: String,
    client: Client,
}

impl NetworkMonitor {
    pub fn new() -> Self {
        NetworkMonitor {
            last_network: String::new(),
            client: Client::new(),
        }
    }

    pub async fn start_monitoring(&mut self) {
        let mut interval = interval(Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            self.check_network_changes().await;
        }
    }

    async fn check_network_changes(&mut self) {
        let (my_ip, current_network) = get_network_info().await;
        
        if self.last_network.is_empty() {
            self.last_network = current_network.clone();
        } else if self.last_network != current_network {
            println!("Network change detected! Reinitializing...");
            self.last_network = current_network.clone();
            self.reinitialize_node(my_ip).await;
        }
    }

    async fn reinitialize_node(&self, my_ip: String) {
        if let Some(genesis_ip) = scan_network().await {
            println!("Found existing network. Connecting to genesis node: {}", genesis_ip);
            let url = format!("http://{}:{}/register-node/{}", genesis_ip, PORT, my_ip);
            if let Err(e) = self.client.post(&url).send().await {
                println!("Failed to register with genesis node: {}", e);
            }
        } else {
            println!("No existing network found. Becoming genesis node.");
            save_genesis_config(my_ip.clone());
            let nodes = vec![my_ip];
            save_node_list(nodes);
        }
    }
}