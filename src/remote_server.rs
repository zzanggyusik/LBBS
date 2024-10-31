use actix_web::web::{Bytes, Json};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use base64::{encode};

use crate::instance::config::{RemoteServerReq, REMOTEIP, REMOTE_SERVER};

pub async fn get_drone_image() -> Bytes {
    let client = Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();
    let remote_ip = REMOTEIP.lock().unwrap().clone();
    let url = format!("{}/drone/video/{}", REMOTE_SERVER, &remote_ip);

    match client.get(&url).send().await {
        Ok(response) => {
            if response.status() == StatusCode::OK {
                response.bytes().await.expect("RESPONSE ERROR")
            }
            else {
                println!("RESPONSE ERROR OR STREAM NOT RUNNING");
                Bytes::new() // Return an empty Bytes object in case of error
            }
        },
        Err(e) => {
            println!("REQUEST ERROR: {}", e);
            Bytes::new() // Return an empty Bytes object in case of error
        },
    }
}

pub async fn get_drone_loc() -> Result<String, &'static str> {
    let client = Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();
    let remote_ip = REMOTEIP.lock().unwrap().clone();
    let url = format!("{}/drone/state/{}", REMOTE_SERVER, remote_ip);

    match client.get(url).send().await {
        Ok(response) => {
            if response.status() == StatusCode::OK {
                let content = response.text().await.expect("RESPONSE ERROR");
                Ok(content)
            }

            else {
                Err("REPONSE ERROR")
            }
        },
        Err(e) => {
            Err("REQUEST ERROR")
        },
    }
}

pub async fn get_car_image() -> Bytes {
    let client = Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();
    let remote_ip = REMOTEIP.lock().unwrap().clone();
    let url = format!("{}/robot/video/{}", REMOTE_SERVER, &remote_ip);

    match client.get(&url).send().await {
        Ok(response) => {
            if response.status() == StatusCode::OK {
                response.bytes().await.expect("RESPONSE ERROR")
            }
            else {
                println!("RESPONSE ERROR OR STREAM NOT RUNNING");
                Bytes::new() // Return an empty Bytes object in case of error
            }
        },
        Err(e) => {
            println!("REQUEST ERROR: {}", e);
            Bytes::new() // Return an empty Bytes object in case of error
        },
    }
}

pub async fn get_car_loc() -> Result<String, &'static str> {
    let client = Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();
    let remote_ip = REMOTEIP.lock().unwrap().clone();
    let url = format!("{}/robot/info/{}", REMOTE_SERVER, remote_ip);

    match client.get(url).send().await {
        Ok(response) => {
            if response.status() == StatusCode::OK {
                let content = response.text().await.expect("RESPONSE ERROR");
                Ok(content)
            }

            else {
                Err("REPONSE ERROR")
            }
        },
        Err(e) => {
            Err("REQUEST ERROR")
        },
    }
}