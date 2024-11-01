use tokio::time::{interval, Duration, timeout};
use std::collections::HashSet;
use crate::config::{self, PORT};
use reqwest::{Client, StatusCode};
use crate::network;

pub struct HeartbeatManager {
    client: Client,
    my_ip: String,
}

impl HeartbeatManager {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))      // 기본 타임아웃
            .connect_timeout(Duration::from_secs(3)) // 연결 타임아웃
            .pool_idle_timeout(Duration::from_secs(30)) // 유휴 연결 타임아웃
            .pool_max_idle_per_host(10)           // 호스트당 최대 유휴 연결 수
            .tcp_keepalive(Duration::from_secs(60)) // TCP keepalive
            .build()
            .unwrap_or_else(|e| {
                eprintln!("Failed to create client, using default: {}", e);
                Client::new()
            });

        HeartbeatManager {
            client,
            my_ip: String::new(),
        }
    }

    pub async fn start_heartbeat(&mut self) {
        tokio::time::sleep(Duration::from_secs(2)).await;
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
            
            let mut retry_count = 0;
            let max_retries = 3;
            
            while retry_count < max_retries {
                if self.is_node_alive(node).await {
                    alive_nodes.insert(node.clone());
                    break;
                }
                retry_count += 1;
                if retry_count < max_retries {
                    // 재시도 전 짧은 대기
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
        
        // 실제 살아있는 다른 노드의 수와 예상 수를 비교
        let actual_alive_count = alive_nodes.len() - 1;  // 자신을 제외한 살아있는 노드 수
        
        if actual_alive_count != expected_alive_count {
            println!("Node check results:");
            println!("- Expected alive nodes: {}", expected_alive_count);
            println!("- Actual alive nodes: {}", actual_alive_count);
            println!("- Current alive nodes: {:?}", alive_nodes);
            println!("- Total nodes in list: {:?}", nodes);

            if actual_alive_count < expected_alive_count {
                println!("Dead nodes detected, updating node list...");
                let new_nodes: Vec<String> = alive_nodes.into_iter().collect();
                config::save_node_list(new_nodes.clone());
                
                // Broadcast updated node list
                self.broadcast_node_list(&new_nodes).await;
            }
        }
    }

    async fn is_node_alive(&self, node: &str) -> bool {
        println!("Checking Nodes State : IP : {}", &node);
        let url = format!("http://{}:{}/heartbeat", node, PORT);
        
        match timeout(
            Duration::from_secs(5),
            self.client.get(&url).send()
        ).await {
            Ok(Ok(response)) => response.status() == StatusCode::OK,
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