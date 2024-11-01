// main.rs
use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use reqwest::{Client, StatusCode};
use actix_cors::Cors;
use std::sync::Mutex;
use std::collections::HashMap;

mod blockchain;
mod instance;
mod network;

use instance::config;
use network::BlockData;

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
    if !nodes.contains(&ip.to_string()) {
        nodes.push(ip.to_string());
        config::save_node_list(nodes.clone());
        
        // Broadcast updated node list to all nodes
        let client = Client::new();
        for node in nodes {
            let url = format!("http://{}:{}/update-nodelist", node, config::PORT);
            let _ = client.post(&url)
                .json(&nodes)
                .send()
                .await;
        }
        HttpResponse::Ok().body("Node registered successfully")
    } else {
        HttpResponse::BadRequest().body("Node already registered")
    }
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
    let client = Client::new();
    
    for node in nodes {
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
        for node in nodes {
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
    config::save_node_list(nodes.into_inner());
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    config::init_config();
    
    // Check if we are genesis node or need to connect to existing network
    if let Some(genesis_ip) = network::scan_network().await {
        // Connect to existing network
        let client = Client::new();
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
        blockchain: Mutex::new(blockchain::Blockchain::new()),
        consensus_votes: Mutex::new(HashMap::new()),
    });
    
    HttpServer::new(move || {
        let cors = Cors::permissive();
        
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
    })
    .bind(format!("0.0.0.0:{}", config::PORT))?
    .run()
    .await
}