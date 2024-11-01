// main.rs
use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use reqwest::{Client, StatusCode};
use actix_cors::Cors;
use std::sync::Mutex;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;

mod blockchain;
mod instance;
mod network;
mod monitor;
mod heartbeat;

use instance::config;
use network::BlockData;
use monitor::NetworkMonitor;
use heartbeat::HeartbeatManager;
use blockchain::Blockchain;

struct AppState {
    blockchain: Mutex<blockchain::Blockchain>,
    consensus_votes: Mutex<HashMap<String, Vec<bool>>>,
}

async fn is_alive() -> impl Responder {
    HttpResponse::Ok().body("Node is alive")
}

async fn genesis_info() -> impl Responder {
    if let Some(genesis_ip) = config::read_genesis_config() {
        HttpResponse::Ok().body(genesis_ip)
    } else {
        HttpResponse::NotFound().finish()
    }
}

async fn register_node(ip: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let mut nodes = config::read_node_list();
    let new_ip = ip.to_string();
    
    if !nodes.contains(&new_ip) {
        println!("Registering new node: {}", new_ip);
        nodes.push(new_ip.clone());
        
        match config::save_node_list(nodes.clone()) {
            Ok(_) => {
                println!("Node list saved successfully. Current nodes: {:?}", nodes);
                
                let client = Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap();
                
                tokio::time::sleep(Duration::from_millis(1000)).await;
                
                let mut broadcast_failures = Vec::new();
                
                for node in &nodes {
                    let url = format!("http://{}:{}/update-nodelist", node, config::PORT);
                    println!("Sending update to: {}", url);
                    
                    match client.post(&url)
                        .json(&nodes)
                        .send()
                        .await {
                        Ok(response) => {
                            if response.status().is_success() {
                                println!("Successfully updated node: {}", node);
                            } else {
                                println!("Failed to update node {} with status: {}", 
                                    node, response.status());
                                broadcast_failures.push((node.clone(), format!("Status: {}", response.status())));
                            }
                        },
                        Err(e) => {
                            println!("Error updating node {}: {}", node, e);
                            broadcast_failures.push((node.clone(), e.to_string()));
                        }
                    }
                }
                
                let message = if broadcast_failures.is_empty() {
                    "Node registered and all nodes updated successfully".to_string()
                } else {
                    format!("Node registered but failed to update some nodes: {:?}", broadcast_failures)
                };
                
                HttpResponse::Ok().json("OK")
            },
            Err(e) => {
                println!("Failed to save node list: {}", e);
                };
                HttpResponse::InternalServerError().json("InternalServerError")
            }
        }
    } else {
        println!("Node {} already registered", new_ip);
        };
        HttpResponse::BadRequest().json("BadRequest")
    }
}

#[derive(Serialize)]
struct NodeInfo {
    ip: String,
    node_list: Vec<String>,
    blockchain: blockchain::Blockchain,
}

// 새로운 get-node-info 엔드포인트 추가
async fn get_node_info(data: web::Data<AppState>) -> impl Responder {
    let my_ip = network::get_network_info().await.0;
    let node_list = config::read_node_list();
    let blockchain = data.blockchain.lock().unwrap().clone();

    let info = NodeInfo {
        ip: my_ip,
        node_list,
        blockchain,
    };

    HttpResponse::Ok().json(info)
}

async fn add_block(
    block_data: web::Json<BlockData>,
    data: web::Data<AppState>
) -> impl Responder {
    // Verify signature
    if !network::verify_signature(&block_data.command, &block_data.sign) {
        return HttpResponse::BadRequest().body("Invalid signature");
    }
    
    // Initialize consensus voting
    let mut consensus_votes = data.consensus_votes.lock().unwrap();
    let vote_id = format!("{}_{}", block_data.ip, block_data.command);
    consensus_votes.insert(vote_id.clone(), Vec::new());
    
    // Request consensus from all nodes
    let nodes = config::read_node_list();
    let client = Client::builder()
        .timeout(Duration::from_millis(1000))
        .build()
        .unwrap();
    
    for node in nodes.iter() {  // .iter()를 사용하여 참조로 순회
        let url = format!("http://{}:{}/consensus", node, config::PORT);
        if let Ok(response) = client.post(&url)
            .json(&block_data)
            .send()
            .await {
            if let Ok(is_valid) = response.json::<bool>().await {
                consensus_votes.get_mut(&vote_id).unwrap().push(is_valid);
            }
        }
    }
    
    // Check consensus
    let votes = consensus_votes.get(&vote_id).unwrap();
    let total_votes = votes.len();
    let positive_votes = votes.iter().filter(|&&v| v).count();
    
    if positive_votes * 2 > total_votes {
        // Add block to blockchain
        let mut blockchain = data.blockchain.lock().unwrap();
        let block_data = blockchain::Data::new(
            block_data.ip.clone(),
            block_data.command.clone(),
            "PENDING".to_string()
        );
        let new_block = blockchain.add_block(block_data);
        
        // Broadcast new block to all nodes
        for node in nodes.iter() {  // .iter()를 사용하여 참조로 순회
            let url = format!("http://{}:{}/update-block", node, config::PORT);
            let _ = client.post(&url)
                .json(&new_block)
                .send()
                .await;
        }
        
        HttpResponse::Ok().body("Block added successfully")
    } else {
        HttpResponse::BadRequest().body("Consensus not reached")
    }
}

async fn consensus(
    block_data: web::Json<BlockData>,
) -> impl Responder {
    let is_valid = network::verify_signature(&block_data.command, &block_data.sign);
    HttpResponse::Ok().json(is_valid)
}

async fn update_block(
    block: web::Json<blockchain::Block>,
    data: web::Data<AppState>
) -> impl Responder {
    let mut blockchain = data.blockchain.lock().unwrap();
    blockchain.update_block(block.into_inner());
    HttpResponse::Ok().finish()
}

async fn update_nodelist(
    nodes: web::Json<Vec<String>>
) -> impl Responder {
    println!({:?}, nodes.clone);
    config::save_node_list(nodes.into_inner());
    HttpResponse::Ok().json("OK")
}

async fn heartbeat() -> impl Responder {
    HttpResponse::Ok().json("OK")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    config::init_config();

    println!("=================================================");
    println!("Blockchain Node Startup");
    println!("=================================================");
    
    let (my_ip, network_prefix) = network::get_network_info().await;
    println!("Interface Detection:");
    println!("- Current Node IP: {}", my_ip);
    println!("- Network Range: {}.1 ~ {}.244", network_prefix, network_prefix);
    println!("=================================================");
    println!("Starting network scan...");

    let mut monitor = NetworkMonitor::new();
    tokio::spawn(async move {
        monitor.start_monitoring().await;
    });

    let genesis_config = config::read_genesis_config();
    let my_ip_clone = my_ip.clone();

    if let Some(genesis_ip) = genesis_config {
        if genesis_ip == my_ip_clone {
            let heartbeat_manager = HeartbeatManager::new();
            tokio::spawn(async move {
                heartbeat_manager.start_heartbeat().await;
            });
        }
    }

    let blockchain = match Blockchain::load_from_file() {
        Ok(chain) => {
            println!("Loaded existing blockchain from file");
            chain
        },
        Err(e) => {
            println!("Failed to load blockchain from file: {}", e);
            println!("Creating new blockchain");
            Blockchain::new()
        }
    };
    
    // Check if we are genesis node or need to connect to existing network
    if let Some(genesis_ip) = network::scan_network().await {
        // Connect to existing network
        let client = Client::builder()
            .timeout(Duration::from_millis(1000))
            .build()
            .unwrap();
        let my_ip = network::get_network_info().await.0;
        let url = format!("http://{}:{}/register-node/{}", genesis_ip, config::PORT, my_ip);
        let _ = client.post(&url).send().await;
    } else {
        // Become genesis node
        let my_ip = network::get_network_info().await.0;
        config::save_genesis_config(my_ip.clone());
        let mut nodes = Vec::new();
        nodes.push(my_ip);
        config::save_node_list(nodes);
    }
    
    let app_state = web::Data::new(AppState {
        blockchain: Mutex::new(blockchain), 
        consensus_votes: Mutex::new(HashMap::new()),
    });
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| {
                true
            })
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allow_any_header()
            .max_age(3600);
        
        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/isalive", web::get().to(is_alive))
            .route("/genesis-info", web::get().to(genesis_info))
            .route("/register-node/{ip}", web::post().to(register_node))
            .route("/add-block", web::post().to(add_block))
            .route("/consensus", web::post().to(consensus))
            .route("/update-block", web::post().to(update_block))
            .route("/update-nodelist", web::post().to(update_nodelist))
            .route("/get-node-info", web::get().to(get_node_info))
    })
    .bind(format!("0.0.0.0:{}", config::PORT))?
    .run()
    .await
}
