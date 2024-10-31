// blockchain.rs
use serde::{Deserialize, Serialize};
use std::{fs::File, time::{SystemTime, UNIX_EPOCH}};
use std::io::prelude::*;
use sha2::{Sha256, Digest};

use crate::instance::config::{self, BlockData, BLOCKCHAINPATH, BLOCKLENGTH, DIFFICULTY};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub data: Vec<Data>,
    pub previous_hash: String,
    pub nonce: u64,
    pub hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Data {
    pub id: String,
    pub command: String,
    pub state: String
}

impl Data {
    pub fn new(id: String, command: String, state: String) -> Self {
        Data {
            id,
            command,
            state
        }
    }
}

impl Block {
    pub fn new(index: u64, data: Vec<Data>, previous_hash: String) -> Self {
        Block {
            index,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            data,
            previous_hash,
            nonce: 0,
            hash: String::new(),
        }
    }

    // Proof of Work Algorithm
    pub fn mine_block(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        self.nonce = 0;

        if self.hash.len() < difficulty {
            self.hash = "0".repeat(difficulty); // 적절한 초기값 설정
        }

        loop {
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&target) {
                break;  // 조건을 만족하면 루프 종료
            }
            self.nonce += 1;  // 조건을 만족하지 않으면 nonce 증가
        }
    }

    fn calculate_hash(&self) -> String {
        let contents = format!("{}{}{:?}{}{}", self.index, self.timestamp, self.data, self.previous_hash, self.nonce);
        let mut hasher = Sha256::new();
        hasher.update(contents);
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub difficulty: usize,
}

impl Blockchain {
    pub fn new() -> Self {
        let mut blockchain = Blockchain {
            blocks: Vec::new(),
            difficulty: DIFFICULTY,
        };

        let mut genesis_data = Vec::new();

        let mut data = Data {
            id : "Genesis".to_owned(),
            command : "Genesis".to_owned(),
            state : "Genesis".to_owned()
        };

        let car1_data = Data {
            id : "192.168.50.207".to_owned(),
            command : "s".to_owned(),
            state : "STAY".to_owned()
        };

        let drone1_data = Data {
            id : "192.168.50.33".to_owned(),
            command : "land".to_owned(),
            state : "STAY".to_owned()
        };

        genesis_data.push(data);
        genesis_data.push(car1_data);
        genesis_data.push(drone1_data);

        let genesis_block = Block::new(0, genesis_data, "0".to_string());
        blockchain.blocks.push(genesis_block);
        blockchain
    }

    pub fn add_block(&mut self, data: Data) -> Block {
        let last_block = self.blocks.last().unwrap();
        let mut new_block_data = last_block.data.clone();
        new_block_data.push(data);

        let mut new_block = Block::new(self.blocks.len() as u64, new_block_data, last_block.hash.clone());
        new_block.mine_block(self.difficulty);
        self.blocks.push(new_block.clone());

        self.update_block_length();

        return new_block.clone();
    }

    pub fn update_block(&mut self, block: Block) {
        self.blocks.push(block);
        self.update_block_length();
    }

    pub fn update_block_length(&mut self) {
        let mut block_length_lock = BLOCKLENGTH.lock().unwrap();
        let block_len = self.blocks.len();

        *block_length_lock = block_len.to_string();
    }

    pub fn is_valid(&self, block: &Block) -> bool {
        for i in 1..self.blocks.len() {
            let current = &self.blocks[i];
            let previous = &self.blocks[i-1];

            if current.hash != Block::calculate_hash(block) {
                return false;
            }

            if current.previous_hash != previous.hash {
                return  false;
            }
        }
        
        true
    }

    pub fn get_last_block(&self) -> &Block {
        self.blocks.last().unwrap()
    }

    pub fn check_data_exist(&mut self, id: String, cmd: String, state: String) -> Block {
        let last_block = self.blocks.last().unwrap().clone();
        let mut _exist_flag = false;

        for data in last_block.data.iter() {
            if &data.id == &id {
                let new_block = self.update_my_data(id.clone(), cmd.clone(), state.clone());
                _exist_flag = true;

                return new_block; 
            }
        }

        let data = Data::new(id.clone(), cmd.clone(), state.clone());
        let new_block = self.add_block(data);

        return new_block;
    }

    fn update_my_data(&mut self, id:String, cmd: String, state: String) -> Block {
        let last_block = self.blocks.last_mut().unwrap();
        for data in &mut last_block.data {
            if data.id == id.clone() {
                if data.command != cmd.clone() {
                    data.command = cmd.clone();
                    data.state = state.clone();
                }
            }
        }
        return self.blocks.last().unwrap().clone();
    }


    // pub fn add_new_transaction(&mut self, tra)


    
}

pub fn check_blockchain_exist() {
    let mut file = File::create(BLOCKCHAINPATH).expect("FILE CREATE ERROR");
    
    let genesis_blockchain = Blockchain::new();
    let data = serde_json::to_string(&genesis_blockchain).expect("JSON ERROR");

    file.write_all(data.as_bytes()).expect("FILE WRITE ERROR");
}

pub fn get_local_blockchain() -> Blockchain {
    let mut file = File::open(BLOCKCHAINPATH).expect("FILE NOT EXIST");
    let mut contents = String::new();

    file.read_to_string(&mut contents).expect("FILE READ ERROR");
    let local_blockchain: Blockchain = serde_json::from_str(&contents).expect("DATA ERROR");

    local_blockchain
}
