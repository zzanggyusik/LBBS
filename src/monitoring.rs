use reqwest::{Client, StatusCode};
use std::result;
use std::time::Duration;
use tokio;

use crate::car::car_send;
use crate::drone::drone_send_cmd;
use crate::{blockchain, remote};
use crate::instance::config::{self, UpdateNode, BLOCKCHAIN, CMD_MONITORING_TIME, GENESIS_NODE, IPADDR, NETWORK_MONITORING_TIME, NODE_TYPE, REMOTEIP, STATE};

pub async fn network_monitoring(init_ip: String) {
    // 주기적으로 IP 확인하여 이전 인터넷 환경과 다를 경우 Genesis 노드에게 업데이트 요청
    // 모니터링 주기는 config의 monitoring_time 을 통해 설정됨
    let mut previous_ip = init_ip;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(NETWORK_MONITORING_TIME));
    loop {
        interval.tick().await;
        let my_ip = local_ip_address::local_ip().unwrap().to_string();

        if previous_ip != my_ip {
            let client = Client::builder()
                .timeout(Duration::from_millis(1000)) // millisecond
                .build()
                .unwrap();
            let body = UpdateNode::new(previous_ip.clone(), my_ip.clone(), NODE_TYPE.lock().unwrap().clone());
            let url = format!("http:{}:{}/delete-node", config::GENESIS_NODE, config::GENESIS_PORT);

            match client.post(url).json(&body).send().await {
                Ok(response) => {
                    if response.status() == StatusCode::OK {
                        println!("Update success");

                        previous_ip = my_ip.clone();

                        let mut ipaddr = IPADDR.lock().unwrap();
                        *ipaddr = my_ip;                        
                    }
                },

                Err(e) => {
                    println!("REQUEST FAIL : {}", e);
                },
            }
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
            let mut my_node= String::new();
            {
                let node_type = NODE_TYPE.lock().unwrap().clone();
                my_node = node_type; 
            }

            if my_node == "drone".to_string() {
                println!("{}", cmd);
                drone_send_cmd(cmd).await;
            }

            else if my_node == "car".to_string() {
                car_send(cmd).await;
            }
            // remote::cmd_start(cmd, node_type).await;
        }
    }
}

pub fn check(init_state: String) -> (bool, String, String, String) {
    let mut my_ip = String::new();
    {
        let ip_lock = IPADDR.lock().unwrap().clone();
        // println!("{}", &ip_lock);
        my_ip = ip_lock;
    }
        
    let mut last_block = blockchain::Block::new(0, vec![blockchain::Data::new(String::new(), String::new(), String::new())], String::new());
    {
        let block_chain_lock = BLOCKCHAIN.lock().unwrap();
        last_block = block_chain_lock.get_last_block().clone();
    }
                    
    for data in last_block.data.iter() {
        // DEBUG
        // if my_ip == GENESIS_NODE {
        //     my_ip = "192.168.50.13".to_owned();
        // }

        //println!("data : {:?}", &data);
                
        if &data.id == &my_ip {
            println!("data : {:?}", &data);
            if &data.state != &data.command{
        
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

                    let result = true;
                    //println!("@@@@@@ {} {} {}", &result, &cur_state, & my_node_type);

                    return (result, cur_state, cmd, my_node_type)
                }
            }
        }
        else {
            // println!("Else Debug Log my ip:{} data ip : {}", &my_ip, &data.id);
        }
    }

    return (false, init_state, String::new(), String::new());
}