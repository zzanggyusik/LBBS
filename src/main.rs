use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder, body};
use blockchain::{check_blockchain_exist, Block, Blockchain, Data};
use image::{ImageOutputFormat, ImageBuffer, RgbImage};
use remote_server::{get_car_image, get_car_loc, get_drone_image, get_drone_loc};
use std::io::{self, Cursor, Write};
use drone::{drone_send_cmd, TELLO};
use instance::config::{BLOCKLENGTH, CMD_PORT, GENESIS_CONFIG_PATH, GENESIS_NODE, GENESIS_PORT, NODE_TYPE, REMOTEIP, REMOTEMODE, STATE_PORT, STREAM_CMD, VIDEO_PORT};
use instance::{config, setup};
use remote::MYLOCATION;
use tokio;

mod p2p;
mod drone;
mod car;
mod blockchain;
mod remote;
mod instance;
mod monitoring;
mod remote_server;
mod network_scanner;

use network_scanner::{read_genesis_config, NetworkScanner};

use config::{Result, Node, Location, UpdateNode, BlockData, BlockInfo};
use config::{BLOCKCHAIN, NODES, IPADDR, PORT, STATE};
mod auth;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut tello_ip = "0.0.0.0".to_owned();
    
    // if local_ip_address::local_ip().unwrap().to_string() == GENESIS_NODE {
    //     tello_ip = "192.168.50.11".to_owned();
    // }

    // else {
    //     tello_ip = local_ip_address::local_ip().unwrap().to_string();
    // }

    //remote::init_tello("drone_id".to_owned(), tello_ip, CMD_PORT, STATE_PORT, VIDEO_PORT).await;

    let genesis_config_path = GENESIS_CONFIG_PATH.to_string();
    let genesis_port = GENESIS_PORT.parse().unwrap();

    let scanner = NetworkScanner::new(genesis_port, genesis_config_path);
    let genesis_ip = scanner.scan_network().await.expect("Failed to scan network");

    let (my_ip, my_port) = setup::setup_mode().await;    
    
    println!("my addr is {}:{}", &my_ip, &my_port);

    if &my_ip != &genesis_ip {
        let nodeinfo = p2p::send(&my_ip, &my_port).await;
        {
            let mut nodes_lock = NODES.lock().unwrap();
            *nodes_lock = nodeinfo.clone();
        }
        println!("Nodes info : {:?}", nodeinfo);
    }
   

    tokio::spawn(monitoring::network_monitoring(my_ip.clone()));
    tokio::spawn(monitoring::cmd_monitoring());

    HttpServer::new(|| {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| {
                true
            })
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .route("/register-node", web::post().to(register_node))
            .route("/get-nodes", web::get().to(get_nodes))
            .route("/consensus", web::post().to(consensus))
            .route("/get-last-blockchain", web::get().to(get_last_blockchain))
            .route("/get-all-blockchain", web::get().to(get_all_blockchain))
            .route("/broadcast-block", web::post().to(broadcast_block))
            .route("/add-block", web::post().to(try_add))
            .route("/broadcast-nodelist", web::post().to(broadcast_nodelist))
            .route("/delete-node", web::post().to(delete_node))
            .route("/get-location", web::get().to(get_location))
            .route("/change-remote-mode", web::post().to(change_remote_mode))
            .route("/get-video", web::get().to(get_video))
            .route("/is-valid", web::get().to(is_valid))
            .route("/isalive", web::get().to(is_alive))
    })
    .bind(format!("0.0.0.0:{}", my_port))?
    .run()
    .await
}

// main.rs에 추가할 엔드포인트 핸들러

async fn is_alive() -> impl Responder {
    match read_genesis_config("./src/instance/genesis_config.txt") {
        Ok(genesis_ip) => HttpResponse::Ok().body(genesis_ip),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

// main() 함수의 App::new() 부분에 추가할 라우트


async fn is_valid() -> impl Responder {
    let my_length = BLOCKLENGTH.lock().unwrap().clone();
    let my_ip = IPADDR.lock().unwrap().clone();

    let response = BlockInfo::new(my_length, my_ip);

    HttpResponse::Ok().json(response)    
}

async fn try_add(req_block_data: web::Json<BlockData>) -> impl Responder {
    p2p::check_chain_valid().await;
    let block_data = req_block_data.clone();
    let check_result: bool = p2p::vote_request(block_data).await;
    let response_message = Result::new(check_result);

    // DEBUG
    println!("VOTE Result : {}", &check_result);
    println!{"REQ msg: {:?}", &req_block_data};

    if check_result {
        // DEBUG
        println!("Ready for Broadcast");
        let mut new_block = Block::new(0, Vec::new() , String::new());
        let mut my_ip = String::new();
        {
            let mut blockchain = BLOCKCHAIN.lock().unwrap();
        
            // DEBUG
            println!("Current Data : {:?}", &blockchain.blocks);

            my_ip = IPADDR.lock().unwrap().clone();
            let cmd = req_block_data.command.clone();
            println!("ip {}", &my_ip);
            println!("cmd : {}", &cmd);

            new_block = blockchain.check_data_exist(req_block_data.id.clone(), cmd, req_block_data.state.clone());        
        }
        
        // DEBUG
        println!("block data : {:?}", &new_block);

        p2p::global_update(new_block, my_ip).await;

        HttpResponse::Ok().json(response_message)
    }

    else {
        HttpResponse::Unauthorized().json(response_message)
    }
}

async fn register_node(node_info: web::Json<Node>) -> impl Responder {
    let node = node_info.into_inner();
    
    let mut nodes = NODES.lock().unwrap();
    
    
    if !nodes.iter().any(|n| n.address == node.address) {
        nodes.push(node.clone());
        print!("Node list updated {:?}", &node);

        p2p::broadcast_nodelist(nodes.clone()).await;
        HttpResponse::Ok().json(&*nodes)
    } else {
        HttpResponse::BadRequest().json(&*nodes)
    }
}

async fn consensus(block_data: web::Json<BlockData>) -> impl Responder {
    let result = p2p::vote(block_data).await;

    let response_message = Result::new(result);
    
    HttpResponse::Ok().json(response_message)
}

async fn get_nodes() -> impl Responder {
    // FUnction For get node list
    let nodes = NODES.lock().unwrap();
    HttpResponse::Ok().json(&*nodes)
}

async fn get_last_blockchain() -> impl Responder {
    // Funciont or get last block of blockchain
    let blockchain = BLOCKCHAIN.lock().unwrap();
    HttpResponse::Ok().json(&*blockchain.get_last_block())
}

async fn get_all_blockchain() -> impl Responder {
    // Get all blockchain datas
    let blockchain = BLOCKCHAIN.lock().unwrap();
    HttpResponse::Ok().json(&*blockchain.blocks)
}

async fn broadcast_block(block_data: web::Json<blockchain::Block>) -> impl Responder {
    // Update Global Blockchain
    let new_block = block_data.into_inner();
    let mut blockchain = BLOCKCHAIN.lock().unwrap();

    blockchain.update_block(new_block);

    println!("New Block Added {:?}", blockchain.get_last_block());

    HttpResponse::Ok().json("Block added")
}

async fn broadcast_nodelist(node_list: web::Json<Vec<Node>>) -> impl Responder {
    let mut pre_nodelist = NODES.lock().unwrap();
    let new_node_list = node_list.into_inner();

    *pre_nodelist = new_node_list;

    HttpResponse::Ok().json("nodelist updated!")
}

async fn delete_node(node_info : web::Json<UpdateNode>) -> impl Responder {
    let node_info = node_info.into_inner();

    let mut node_list = Vec::new();

    {
        node_list = NODES.lock().unwrap().clone();
    }

    node_list.retain(|node| node.address != node_info.delete_ip);

    // let new_node = Node::new(node_info.update_ip, node_info.node_type);
    
    // node_list.push(new_node);

    p2p::broadcast_nodelist(node_list.clone()).await;

    HttpResponse::Ok().json("Updated!")
}

async fn get_location() -> impl Responder {
    // Location xyz
    //let (mut x, mut y, mut z) = ("00.00".to_owned(), "00.00".to_owned(), "00.00".to_owned());

    // let reponse = MYLOCATION.lock().unwrap().clone();
    let mut my_type = String::new();

    {
        my_type = NODE_TYPE.lock().unwrap().clone();
    }

    if my_type == "drone" {
        let result = get_drone_loc().await;
        HttpResponse::Ok().json(result)
    }

    else if my_type == "car" {
        let result = get_car_loc().await;
        HttpResponse::Ok().json(result)
    }

    else {
        HttpResponse::NotFound().json("Not Found")
    }
    

    // println!("200 : {:?}", reponse);
    
    
}

async fn change_remote_mode(request : web::Json<BlockData>) -> impl Responder {
    let req_data = request.into_inner();
    println!("{:?}", req_data.clone());

    let check_result = p2p::vote_request(req_data.clone()).await;
    println!("Vote Result : {}", &check_result);
    
    if check_result {
        let mut new_block = Block::new(0, Vec::new(), String::new());
        {
            let mut blockchain = BLOCKCHAIN.lock().unwrap();
            new_block = blockchain.check_data_exist(req_data.id.clone(), req_data.command.clone(), req_data.state.clone());  
        }
        p2p::global_update(new_block, IPADDR.lock().unwrap().clone()).await;

        let change_result = p2p::change_remote_mode().await;

        if change_result {
            let response_message = Result::new(true);

            let my_type = NODE_TYPE.lock().unwrap().clone();
            if my_type == "drone" {
                drone_send_cmd("streamon".to_string()).await;
            }
            HttpResponse::Ok().json(response_message)
        }

        else {
            let response_message = Result::new(false);
            HttpResponse::BadRequest().json(response_message)
        }
    }

    else {
        let response_message = Result::new(false);
        HttpResponse::Unauthorized().json(response_message)
    }   
}

async fn get_video() -> impl Responder {
    // TODO : 현기 서버로 넘김
    
    let remote_state = REMOTEMODE.lock().unwrap().clone();
    if remote_state {
        
        let my_type = NODE_TYPE.lock().unwrap().clone();
        if my_type == "drone" {
            

            let result = get_drone_image().await;
            println!("Video : {:?}", &result);
            HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(result)
        }

        else if my_type == "car" {
            let result = get_car_image().await;
            println!("Video : {:?}", &result);
            HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(result)
        }

        else {
            println!("Failed");
            HttpResponse::NotFound().json("Not Found")
        }
        
    }
    else { 
        HttpResponse::BadRequest().json("REMOTE MODE OFF")
    }
}
