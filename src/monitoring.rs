// monitoring.rs
use reqwest::{Client, StatusCode};
use std::result;
use std::time::Duration;
use tokio;

use crate::car::car_send;
use crate::drone::drone_send_cmd;
use crate::{blockchain, remote};
use crate::instance::config::{self, UpdateNode, BLOCKCHAIN, CMD_MONITORING_TIME, GENESIS_CONFIG_PATH, IPADDR, NETWORK_MONITORING_TIME, NODE_TYPE, REMOTEIP, STATE, GENESIS_PORT};
use crate::network_scanner::{NetworkScanner, read_genesis_config};

pub async fn network_monitoring(init_ip: String) {
    let mut previous_ip = init_ip;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(NETWORK_MONITORING_TIME));
    let interface_name = "enp0s8".to_string();  // 사용할 네트워크 인터페이스

    loop {
        interval.tick().await;
        if let Some(my_ip) = NetworkScanner::get_interface_ip(&interface_name) {
            let my_ip = my_ip.to_string();

            if previous_ip != my_ip {
                println!("Network change detected! Previous IP: {}, New IP: {}", previous_ip, my_ip);
                
                // 네트워크 스캔 실행
                let scanner = NetworkScanner::new(
                    GENESIS_PORT.parse().unwrap(),
                    GENESIS_CONFIG_PATH.to_string(),
                    interface_name.clone()
                );

                match scanner.scan_network().await {
                    Ok(genesis_ip) => {
                        println!("Found genesis node at: {}", genesis_ip);
                        
                        if my_ip == genesis_ip {
                            println!("Becoming genesis node");
                            // 제네시스 노드가 된 경우 관련 설정 업데이트
                            let mut ipaddr = IPADDR.lock().unwrap();
                            *ipaddr = my_ip.clone();
                        } else {
                            println!("Connecting to existing genesis node");
                            // 기존 제네시스 노드에 재연결
                            let client = Client::builder()
                                .timeout(Duration::from_millis(10000))
                                .build()
                                .unwrap();
                            
                            let body = config::Node {
                                address: format!("{}:{}", my_ip, GENESIS_PORT),
                                node_type: NODE_TYPE.lock().unwrap().clone()
                            };

                            let url = format!("http://{}:{}/register-node", genesis_ip, GENESIS_PORT);
                            
                            match client.post(&url).json(&body).send().await {
                                Ok(response) => {
                                    if response.status() == StatusCode::OK {
                                        println!("Successfully registered with genesis node");
                                        let mut ipaddr = IPADDR.lock().unwrap();
                                        *ipaddr = my_ip.clone();
                                    } else {
                                        println!("Failed to register with genesis node: {}", response.status());
                                    }
                                },
                                Err(e) => {
                                    println!("Failed to connect to genesis node: {}", e);
                                }
                            }
                        }
                        
                        previous_ip = my_ip;
                    },
                    Err(e) => {
                        println!("Network scan failed: {}", e);
                    }
                }
            }
        } else {
            println!("Failed to get IP from interface {}", interface_name);
        }
    }
}

pub async fn cmd_monitoring() {
    let mut init_state = "Unknown".to_owned();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(CMD_MONITORING_TIME));
    loop {
        interval.tick().await;

        let (result, cur_state, cmd, node_type) = check(init_state.clone());

        if result {
            init_state = cur_state;
            let mut my_node = String::new();
            {
                let node_type = NODE_TYPE.lock().unwrap().clone();
                my_node = node_type; 
            }

            if my_node == "drone".to_string() {
                println!("{}", cmd);
                drone_send_cmd(cmd).await;
            } else if my_node == "car".to_string() {
                car_send(cmd).await;
            }
        }
    }
}

pub fn check(init_state: String) -> (bool, String, String, String) {
    let mut my_ip = String::new();
    {
        let ip_lock = IPADDR.lock().unwrap().clone();
        my_ip = ip_lock;
    }
        
    let mut last_block = blockchain::Block::new(0, vec![blockchain::Data::new(String::new(), String::new(), String::new())], String::new());
    {
        let block_chain_lock = BLOCKCHAIN.lock().unwrap();
        last_block = block_chain_lock.get_last_block().clone();
    }
                    
    for data in last_block.data.iter() {
        if &data.id == &my_ip {
            println!("data : {:?}", &data);
            if &data.state != &data.command {
                if init_state != data.state {
                    let cmd = data.command.clone();
                    let cur_state = data.state.clone();
                    let mut my_state = STATE.lock().unwrap();
                    *my_state = init_state.clone();

                    let mut my_node_type = String::new();
                    {
                        let node_type = NODE_TYPE.lock().unwrap().clone();
                        my_node_type = node_type;
                    }

                    return (true, cur_state, cmd, my_node_type)
                }
            }
        }
    }

    return (false, init_state, String::new(), String::new());
}
