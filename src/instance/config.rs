use lazy_static::lazy_static;
use std::sync::Mutex;
use std::fs::{File, create_dir_all};
use std::io;
use std::io::{Write, Read};
use std::path::Path;

pub const BLOCKCHAINPATH: &str = "./data/blockchain.json";
pub const GENESIS_CONFIG_PATH: &str = "./config/genesis_config.txt";
pub const NODE_LIST_PATH: &str = "./config/nodelist.txt";
pub const PORT: &str = "9999";
pub const DIFFICULTY: usize = 4;

lazy_static! {
    pub static ref BLOCKLENGTH: Mutex<String> = Mutex::new(String::new());
}

pub fn init_config() {
    // Create necessary directories
    create_dir_all("./data").expect("Failed to create data directory");
    create_dir_all("./config").expect("Failed to create config directory");
    create_dir_all("./key").expect("Failed to create key directory");
}

pub fn save_genesis_config(ip: String) {
    let mut file = File::create(GENESIS_CONFIG_PATH).expect("Failed to create genesis config");
    file.write_all(ip.as_bytes()).expect("Failed to write genesis IP");
}

pub fn read_genesis_config() -> Option<String> {
    if let Ok(mut file) = File::open(GENESIS_CONFIG_PATH) {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            Some(contents)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn save_node_list(nodes: Vec<String>) -> io::Result<()> {
    let contents = nodes.join("\n");
    let mut file = File::create(NODE_LIST_PATH)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

pub fn read_node_list() -> Vec<String> {
    if let Ok(mut file) = File::open(NODE_LIST_PATH) {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            contents.lines().map(String::from).collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}
