use pnet::packet::ip::IpNextHeaderProtocols::St;
use serde::{Serialize, Deserialize};
use std::sync::Mutex;
use lazy_static::lazy_static;
use crate::{auth, p2p, blockchain};

pub static DIFFICULTY: usize = 1;
pub static CHECKING_MSG: &str = "check_my_auth";
pub static KEY_PATH: &str = "src/instance/key";
pub static GENESIS_NODE: &str = "192.168.50.113";
pub static GENESIS_PORT: &str = "9999";
pub static NETWORK_MONITORING_TIME: u64 = 10000;
pub static CMD_MONITORING_TIME: u64 = 2000;
pub static CMD_PORT: u16 = 8889;
pub static STATE_PORT: u16 = 8890;
pub static VIDEO_PORT: u16 = 11111;
pub static INIT_CMD: &str = "command";
pub static STREAM_CMD: &str = "streamon";
pub static NAL_UNIT:[u8; 4]= [0, 0, 0, 1];
pub static BLOCKCHAINPATH : &str = "src/instance/data";
pub static REMOTE_SERVER: &str = "http://192.168.50.254:17148";
pub static REMOTE_CONFIG_FILE_PATH: &str = "./remote_config";
pub static GENESIS_CONFIG_PATH: &str = "./src/instance/genesis_config.txt";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockData {
    pub id : String,
    pub command : String,
    pub sign : Vec<u8>,
    pub node_type : String,
    pub state: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vote {
    pub addr : String,
    pub result : bool,
}

impl Vote {
    pub fn new(addr: String, result: bool) -> Self {
        Vote{
            addr,
            result
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Result {
    pub result : bool
}

impl Result {
    pub fn new(result: bool) -> Self {
        Result{
            result
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateNode {
    pub delete_ip : String,
    pub update_ip : String,
    pub node_type : String
}

impl UpdateNode {
    pub fn new(delete_ip: String, update_ip: String, node_type: String) -> Self {
        UpdateNode {
            delete_ip,
            update_ip,
            node_type
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockInfo {
    pub length: String,
    pub addr: String
}

impl BlockInfo {
    pub fn new(length: String, addr: String) -> Self {
        BlockInfo {
            length,
            addr
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoteServerReq {
    pub ip: String,
    pub cmd: String,
    pub description: String
}

impl RemoteServerReq {
    pub fn new(ip: String, cmd: String, description: String) -> Self {
        RemoteServerReq {
            ip,
            cmd,
            description
        }
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub address: String,
    pub node_type: String,
}

impl Node {
    pub fn new(address: String, node_type: String) -> Self {
        Node{
            address,
            node_type,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Location {
    pub x: String,
    pub y: String,
    pub z: String
}

impl Location {
    pub fn new(x: String, y: String, z: String) -> Self {
        Location{
            x,
            y,
            z
        }
    }
}

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct RemoteMode {
//     pub id : String,
//     pub command : String,
//     pub sign : Vec<u8>,
//     pub node_type: String
// }

static Key_type: &str = "usb";


lazy_static! {
    pub static ref BLOCKCHAIN: Mutex<blockchain::Blockchain> = Mutex::new(blockchain::Blockchain::new());
    pub static ref NODES: Mutex<Vec<Node>> = Mutex::new(Vec::new());
    pub static ref IPADDR: Mutex<String> = Mutex::new(local_ip_address::local_ip().unwrap().to_string());
    pub static ref PORT: Mutex<String> = Mutex::new(GENESIS_PORT.to_string());
    pub static ref NODE_TYPE: Mutex<String> = Mutex::new(auth::get_my_node_type());
    pub static ref KEYTYPE: Mutex<String> = Mutex::new(Key_type.to_string());
    pub static ref STATE: Mutex<String> = Mutex::new(String::new());
    pub static ref REMOTEMODE: Mutex<bool> = Mutex::new(true);
    pub static ref BLOCKLENGTH: Mutex<String> = Mutex::new(String::new());
    pub static ref REMOTEIP: Mutex<String> = Mutex::new(String::new());
}
