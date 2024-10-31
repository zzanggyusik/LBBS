use std::env;
use std::io::{self, BufRead, BufReader};
use std::fs::File;
use crate::instance::config::REMOTE_CONFIG_FILE_PATH;
use crate::{instance::config::{self, NODE_TYPE, REMOTEIP}, IPADDR, PORT};

use super::config::REMOTEMODE;

pub fn check_if_container() -> String {
    let container_name = env::var("CONTAINER_NAME").unwrap_or_else(|_| "Unknown".to_string());
    container_name
}

pub fn container_setting() -> String {
    let container_port = env::var("PORT").unwrap_or_else(|_| "Unknown".to_string());
    container_port
}

pub async fn setup_mode() -> (String, String) {
    let mut my_ip = IPADDR.lock().unwrap();
    let mut my_port = PORT.lock().unwrap();

    let container_name = check_if_container();
    
    if container_name != "Unknown" {
        println!("Change to Container mode");

        let ip_name = container_name;
        let mut port = container_setting();

        if port == "Unknown" {
            port = config::GENESIS_PORT.to_owned();
            println!("PORT ENV NOT FOUND PORT SETUP TO 9999");
        }

        *my_ip = ip_name;
        *my_port = port;
    }

    else {
        println!("Local MODE");
    }

    return (my_ip.clone(), my_port.clone());
}

pub async fn clear_remote_mode() {
    let mut remote_mode_lock = REMOTEMODE.lock().unwrap();

    *remote_mode_lock = false;
}

pub fn local_node_setup() {
    let file_path = REMOTE_CONFIG_FILE_PATH;
    let file = File::open(file_path).expect("CONFIG FILE NOT FOUND");

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.expect("Failed to read file");
        let parts: Vec<&str> = line.split(':').map(|s|s. trim()).collect();

        if parts.len() != 2 {
            println!("CHECK FILE!! is Not Correct form");
            continue;
        }

        match parts[0] {
            "node_type" => {
                let mut node_tpye = NODE_TYPE.lock().unwrap();
                println!("Node Type : {}", &parts[1]);
                *node_tpye = parts[1].to_string();
            },
            "remote_ip" => {
                let mut remote_ip = REMOTEIP.lock().unwrap();
                println!("Remote IP : {}", &parts[1]);
                *remote_ip = parts[1].to_string();
            },
            _ => {}
        }
    }
}

pub fn genesis_node_setup() {
    let input_type = "Genesis";
    let remote_ip = "Genesis";

    {
        let mut node_type = NODE_TYPE.lock().unwrap();
        *node_type = input_type.to_string();

        let mut remote_addr = REMOTEIP.lock().unwrap();
        *remote_addr = remote_ip.to_string();
    }
}