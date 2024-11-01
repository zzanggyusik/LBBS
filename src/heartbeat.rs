use tokio::time::{interval, Duration, timeout};
use std::collections::HashSet;
use crate::config::{self, PORT};
use reqwest::Client;

pub struct HeartbeatManager {
    client: Client,
}

impl HeartbeatManager {
    pub fn new() -> Self {
        HeartbeatManager {
            client: Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap(),
        }
    }

    pub async fn start_heartbeat(&self) {
        let mut interval = interval(Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            self.check_nodes().await;
        }
    }

    async fn check_nodes(&self) {
        let nodes = config::read_node_list();
        let mut alive_nodes = HashSet::new();
        
        for node in nodes.iter() {
            if self.is_node_alive(node).await {
                alive_nodes.insert(node.clone());
            }
        }
        
        // Update node list if any node is dead
        if alive_nodes.len() != nodes.len() {
            println!("Dead nodes detected, updating node list...");
            let new_nodes: Vec<String> = alive_nodes.into_iter().collect();
            config::save_node_list(new_nodes.clone());
            
            // Broadcast updated node list
            self.broadcast_node_list(&new_nodes).await;
        }
    }

    async fn is_node_alive(&self, node: &str) -> bool {
        let url = format!("http://{}:{}/heartbeat", node, PORT);
        match timeout(
            Duration::from_secs(5),
            self.client.get(&url).send()
        ).await {
            Ok(Ok(response)) => response.status().is_success(),
            _ => false
        }
    }

    async fn broadcast_node_list(&self, nodes: &Vec<String>) {
        for node in nodes {
            let url = format!("http://{}:{}/update-nodelist", node, PORT);
            if let Err(e) = self.client.post(&url).json(nodes).send().await {
                println!("Failed to update node list for {}: {}", node, e);
            }
        }
    }
}
