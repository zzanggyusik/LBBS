use tokio::time::{interval, Duration, timeout};
use std::collections::HashSet;
use crate::config::{self, PORT};
use reqwest::Client;
use crate::network;

pub struct HeartbeatManager {
    client: Client,
    my_ip: String,
}

impl HeartbeatManager {
    pub fn new() -> Self {
        HeartbeatManager {
            client: Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap(),
            my_ip: String::new(),
        }
    }

    pub async fn start_heartbeat(&mut self) {
        self.my_ip = network::get_network_info().await.0;
        let mut interval = interval(Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            self.check_nodes().await;
        }
    }

    async fn check_nodes(&self) {
        let nodes = config::read_node_list();
        let mut alive_nodes = HashSet::new();
        let mut expected_alive_count = 0;  // 예상되는 살아있는 노드 수
        
        // 자신의 IP는 항상 살아있는 노드 목록에 포함
        alive_nodes.insert(self.my_ip.clone());
        
        for node in nodes.iter() {
            if node == &self.my_ip {
                continue;  // 자신의 IP는 카운트에서 제외
            }
            
            expected_alive_count += 1;  // 자신 이외의 노드마다 카운트 증가
            
            if self.is_node_alive(node).await {
                alive_nodes.insert(node.clone());
            }
        }
        
        // 실제 살아있는 다른 노드의 수와 예상 수를 비교
        let actual_alive_count = alive_nodes.len() - 1;  // 자신을 제외한 살아있는 노드 수
        
        if actual_alive_count != expected_alive_count {
            println!("Dead nodes detected, updating node list...");
            println!("Expected alive nodes: {}, Actual alive nodes: {}", 
                    expected_alive_count, actual_alive_count);
            let new_nodes: Vec<String> = alive_nodes.into_iter().collect();
            config::save_node_list(new_nodes.clone());
            
            // Broadcast updated node list
            self.broadcast_node_list(&new_nodes).await;
        }
    }

    async fn is_node_alive(&self, node: &str) -> bool {
        let url = format!("http://{}:{}/heartbeat", node.clone(), PORT);
        println!("Checking Nodes State : IP : {}", node);
        match timeout(
            Duration::from_secs(60),
            self.client.get(&url).send()
        ).await {
            Ok(Ok(response)) => response.status().is_success(),
            _ => false
        }
    }

    async fn broadcast_node_list(&self, nodes: &Vec<String>) {
        for node in nodes {
            if node == &self.my_ip {
                continue;
            }
            
            let url = format!("http://{}:{}/update-nodelist", node, PORT);
            if let Err(e) = self.client.post(&url).json(nodes).send().await {
                println!("Failed to update node list for {}: {}", node, e);
            }
        }
    }
}