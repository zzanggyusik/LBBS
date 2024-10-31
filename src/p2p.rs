// p2p.rs
use actix_web::web::Json;
use reqwest::{Client, StatusCode};
use std::time::Duration;
use std::{result, vec};

use local_ip_address;
use tokio;

use crate::blockchain::{Block, Blockchain};
use crate::instance::config::{self, Node, UpdateNode, BLOCKLENGTH, GENESIS_CONFIG_PATH, GENESIS_PORT, KEYTYPE, PORT};
use crate::instance::config::{NODES, BLOCKCHAIN, IPADDR, NODE_TYPE};
use crate::instance::setup::{clear_remote_mode, genesis_node_setup, local_node_setup};
use crate::{auth, blockchain, get_nodes};
use crate::network_scanner::read_genesis_config;

pub async fn send(ip: &str, port: &str) -> Vec<config::Node> {
    let my_ip = ip;
    let my_port = port;

    // 제네시스 노드 IP 읽기
    let genesis_ip = match read_genesis_config(GENESIS_CONFIG_PATH) {
        Ok(ip) => ip,
        Err(e) => {
            println!("Failed to read genesis config: {}", e);
            return vec![];
        }
    };

    if my_ip == &genesis_ip {
        let my_address = format!("{}:{}", my_ip, my_port);
        println!("I am Genesis Node, Network now OPENED!");
        let genesis_node = config::Node::new(my_address, "Genesis".to_owned());
        let mut genesis_vec = Vec::new();
        genesis_vec.push(genesis_node);

        genesis_node_setup();

        return genesis_vec;
    } else {
        local_node_setup();

        let port = GENESIS_PORT;
        let body = config::Node {
            address: format!("{}:{}", my_ip, my_port),
            node_type: NODE_TYPE.lock().unwrap().clone()
        };
        let url = format!("http://{}:{}/register-node", &genesis_ip, &port);
        let client = Client::builder()
            .timeout(Duration::from_millis(10000))
            .build()
            .unwrap();

        println!("Register Start : {:?}", &body);

        match client.post(&url).json(&body).send().await {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    match response.json::<Vec<config::Node>>().await {
                        Ok(node_list) => {
                            check_chain_valid().await;
                            return node_list;
                        }
                        Err(e) => {
                            println!("RESPONSE DATA ERROR {}", e);
                            return vec![];
                        }
                    }
                } else {
                    match response.json::<Vec<config::Node>>().await {
                        Ok(node_list) => {
                            println!("NODE ALREADY EXIST... UPDATE NODE LIST!");
                            check_chain_valid().await;
                            return node_list;
                        }
                        Err(e) => {
                            println!("RESPONSE ERROR {}", e);
                            return vec![];
                        }
                    }
                }
            }
            Err(e) => {
                println!("REQUEST ERROR {}", e);
                return vec![];
            }
        }
    }
}

pub async fn broadcast_nodelist(nodelist: Vec<config::Node>) {
    let client = Client::builder()
        .timeout(Duration::from_millis(10000))
        .build()
        .unwrap(); 
    let body = nodelist.clone();
    let my_ip = IPADDR.lock().unwrap().clone();
    let my_port = PORT.lock().unwrap().clone();
    let my_addr = format!("{}:{}", my_ip, my_port);

    for node in nodelist {
        if &node.address != &my_addr {
            let url = format!("http://{}/broadcast-nodelist", &node.address);
            match client.post(&url).json(&body).send().await {
                Ok(response) => {
                    if response.status() == StatusCode::OK {
                        println!("update success to {}", &node.address)
                    }
                },
                Err(e) => {
                    println!("Update Fail REQUEST ERROR : {}", e)
                },
            }
        }
    }
}

pub async fn check_chain_valid() {
    let genesis_ip = match read_genesis_config(GENESIS_CONFIG_PATH) {
        Ok(ip) => ip,
        Err(e) => {
            println!("Failed to read genesis config: {}", e);
            return;
        }
    };
    let port = GENESIS_PORT;

    let url = format!("http://{}:{}/is-valid", genesis_ip, port);
    let client = Client::builder()
        .timeout(Duration::from_millis(10000))
        .build()
        .unwrap();

    match client.get(&url).send().await {
        Ok(response) => {
            if response.status() == StatusCode::OK {
                match response.json::<config::BlockInfo>().await {
                    Ok(blockinfo) => {
                        let my_len = BLOCKLENGTH.lock().unwrap().clone();
                        if blockinfo.length > my_len {
                            get_all_blockchain().await;
                        }
                    },
                    Err(e) => {
                        println!("CHECK RESPONSE TYPE ERROR : {}", e);
                    },
                }
            }
        },
        Err(e) => {
            println!("CHECK RESPONSE ERROR : {}", e);
        },
    }
}

async fn get_all_blockchain() {
    let genesis_ip = match read_genesis_config(GENESIS_CONFIG_PATH) {
        Ok(ip) => ip,
        Err(e) => {
            println!("Failed to read genesis config: {}", e);
            return;
        }
    };
    let port = GENESIS_PORT;

    let url = format!("http://{}:{}/get-all-blockchain", genesis_ip, port);
    let client = Client::builder()
        .timeout(Duration::from_millis(10000))
        .build()
        .unwrap();

    match client.get(url).send().await {
        Ok(response) => {
            if response.status() == StatusCode::OK {
                match response.json::<Vec<Block>>().await {
                    Ok(blocks) => {
                        let mut blockchain_lock = BLOCKCHAIN.lock().unwrap();
                        blockchain_lock.blocks = blocks;
                    },
                    Err(e) => {
                        println!("RESPONSE DATA TYPE ERROR : {}", e);
                    },
                }
            }
        },
        Err(e) => {
            println!("RESPONSE ERROR : {}", e);
        },
    }
}

pub async fn vote(block_data: Json<config::BlockData>) -> bool {
    let key_type = KEYTYPE.lock().unwrap().clone();
    auth::check_auth_valid(&key_type, &block_data.command, &block_data.sign)
}

pub async fn vote_request(block_data: config::BlockData) -> bool {
    let client = Client::builder()
        .timeout(Duration::from_millis(10000))
        .build()
        .unwrap();

    let node_list = NODES.lock().unwrap();
    let mut result_list = Vec::new();
    
    for node in &*node_list {
        let url = format!("http://{}/consensus", &node.address);
        match client.post(&url).json(&block_data).send().await {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    match response.json::<config::Result>().await {
                        Ok(response_data) => {
                            let data = config::Vote {
                                addr: node.address.clone(),
                                result: response_data.result,
                            };
                            result_list.push(data)
                        },
                        Err(e) => {
                            println!("RESPONSE DATA ERROR {}", e)
                        },
                    }
                }
            },
            Err(e) => {
                println!("REQUEST ERROR NODE NOT FOUND {}", e)
            },
        }
    }
    
    calculate_vote_result(result_list)
}

pub fn calculate_vote_result(result_list: Vec<config::Vote>) -> bool {
    let total_votes = result_list.len();
    let true_count = result_list.iter().filter(|&vote| vote.result).count();

    let percent = (total_votes as f64) * 0.51;
    true_count as f64 > percent
}

pub async fn global_update(block_data: blockchain::Block, ip: String) {
    let my_ip = ip;
    let my_port = GENESIS_PORT;
    let my_addr = format!("{}:{}", my_ip, my_port);
    let body = &block_data;

    let client = Client::builder()
        .timeout(Duration::from_millis(10000))
        .build()
        .unwrap();
   
    let node_list = NODES.lock().unwrap();
    println!("Node list : {:?}", &node_list);
    
    for node in &*node_list {
        if &node.address != &my_addr {
            let url = format!("http://{}/broadcast-block", &node.address);
            match client.post(&url).json(&body).send().await {
                Ok(response) => {
                    if response.status() == StatusCode::OK {
                        println!("{} update Success", &node.address)
                    } else {
                        println!("{} update Failed", &node.address)
                    }
                },
                Err(e) => {
                    println!("REQUEST FAIL {}", e)
                }
            }
        }
    }
}

pub async fn change_remote_mode() -> bool {
    let my_ip = IPADDR.lock().unwrap().clone();
    let my_type = NODE_TYPE.lock().unwrap().clone();
    let my_port = PORT.lock().unwrap().clone();

    let node_info = UpdateNode::new(format!("{}:{}", my_ip, my_port), "None".to_owned(), my_type);

    let genesis_ip = match read_genesis_config(GENESIS_CONFIG_PATH) {
        Ok(ip) => ip,
        Err(e) => {
            println!("Failed to read genesis config: {}", e);
            return false;
        }
    };

    let client = Client::builder()
        .timeout(Duration::from_millis(10000))
        .build()
        .unwrap();

    let url = format!("http://{}:{}/delete-node", genesis_ip, GENESIS_PORT);

    match client.post(url).json(&node_info).send().await {
        Ok(_) => {
            // Remote mode start
            true
        },
        Err(e) => {
            println!("DELETE ERROR : IS GENESIS ALIVE? : {}", e);
            false
        },
    }
}
