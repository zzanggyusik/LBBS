use reqwest::{Client, StatusCode};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot, Mutex};
use once_cell::sync::Lazy;
use tokio::task;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use serde_json::{json, to_string, Value};
use actix::{Actor, StreamHandler};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;

use crate::instance::config::{Location, RemoteServerReq, NAL_UNIT, REMOTEIP, REMOTE_SERVER};
use crate::remote::MYLOCATION;

pub async fn car_send(cmd:String) {
    let client = Client::builder()
        .timeout(Duration::from_millis(15000))
        .build()
        .unwrap();
    let url = format!("{}/robot/control/",REMOTE_SERVER.clone());
    let id = REMOTEIP.lock().unwrap().clone();
    let cmd = cmd;
    let description = String::new();
    let body = RemoteServerReq::new(id, cmd, description);

    match client.post(url).json(&body).send().await {
        Ok(response) => {
            if response.status() == StatusCode::OK {
                println!("REMOTE CMD SEND");
            }
        },
        Err(e) => {
            println!("REQUEST ERROR : {}",e);
        },
    }
}