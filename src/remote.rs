use tokio::time::{sleep, Duration};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;

use crate::drone::{self, Tello, TELLO};
use crate::instance::config::{Location, CMD_PORT, INIT_CMD, NODE_TYPE, STATE_PORT, VIDEO_PORT, STREAM_CMD};

use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref MYLOCATION: Mutex<Location> = Mutex::new(Location::new("00.00".to_owned(), "00.00".to_owned(), "00.00".to_owned()));
}

pub async fn init_tello(drone_id: String, drone_ip: String, cmd_port: u16, state_port: u16, video_port: u16) {
    {
        let tello = Tello::new(drone_id, drone_ip, cmd_port, state_port, video_port).await;
        let mut tello_lock = TELLO.lock().await;

        *tello_lock = Some(tello);
    }

    set_tello_command_mode().await;
    cmd_start(STREAM_CMD.to_string(), "donre".to_string()).await;
}

async fn set_tello_command_mode() {
    let tello_lock = TELLO.lock().await;
    if let Some(tello) = &*tello_lock {
        tello.send_command(INIT_CMD.to_string()).await;
    }
}

pub async fn cmd_start(cmd: String, node_type: String) {
    let tello_lock = TELLO.lock().await;
    if let Some(tello) = &*tello_lock {

        /*
        Usable Command
        takeoff, land, streamon, streamoff
        */
        
        // DEBUG

        if cmd == "takeoff".to_string() {
            tello.send_command("takeoff".to_string()).await;

            sleep(Duration::from_secs(3)).await;

            tello.send_command("land".to_string()).await;
            
            sleep(Duration::from_secs(20)).await;
        }

        tello.send_command(cmd).await;
        sleep(Duration::from_millis(100)).await;
    }
}


