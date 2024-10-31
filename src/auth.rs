use openssl::rsa::Rsa;
use openssl::pkey::PKey;
use openssl::sign::{Signer, Verifier};
use openssl::x509::extension;
use std::fs::{self, File};
use std::io::Read;
use openssl::hash::MessageDigest;
use walkdir::WalkDir;
use std::path::Path;
use std::io;

use crate::instance::{self, config};

/*
auth.rs는 인증 및 서명과 관련된 함수로 이를 통해 자신이 어떠한 노드인지 확인하고
서명 option에 따라 서명을 진행함
*/

pub fn get_my_node_type() -> String {
    let mut key = "";

    if let Ok(entries) = fs::read_dir(config::KEY_PATH) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(extension) = path.extension(){
                    if extension == "pem" {
                        if let Some(file_name) = path.file_stem() {
                            if let Some(name) = file_name.to_str() {
                                let result = match  name {
                                    "usb" => "usb",
                                    "drone" => "drone",
                                    "controller" => "controller",
                                    "usb_private_key" => "user",
                                    _ => "unknown"
                                };
                                key = result;
                            }
                        }
                    }
                }
            }
        }
    }

    key.to_string()
}

// 서명 생성과 함께 데이터를 반환하는 함수
pub fn send_auth(option: &str, data: &str) -> Vec<u8> {
    let mut private_key_file = File::open(format!("./src/instance/{}_private_key.pem", option)).expect("Unable to open private key file");
    let mut private_key_pem = Vec::new();
    private_key_file.read_to_end(&mut private_key_pem).expect("Failed to read private key file");
    let private_key = Rsa::private_key_from_pem(&private_key_pem).expect("Failed to parse private key");
    let pkey = PKey::from_rsa(private_key).expect("Failed to convert RSA to PKey");

    let mut signer = Signer::new(MessageDigest::sha256(), &pkey).expect("Failed to create signer");
    signer.update(data.as_bytes()).expect("Failed to write data to signer");
    let signature = signer.sign_to_vec().expect("Failed to sign data");

    signature
}

// 서명을 검증하고 결과를 반환하는 함수
pub fn check_auth_valid(option: &str, data: &str, signature: &Vec<u8>) -> bool {
    let option = option.trim();
    let mut public_key_file = File::open(format!("./src/instance/key/{}.pem", option)).expect("Unable to open public key file");
    let mut public_key_pem = Vec::new();
    public_key_file.read_to_end(&mut public_key_pem).expect("Failed to read public key file");
    let public_key = Rsa::public_key_from_pem(&public_key_pem).expect("Failed to parse public key");
    let pkey = PKey::from_rsa(public_key).expect("Failed to convert RSA to PKey");

    let mut verifier = Verifier::new(MessageDigest::sha256(), &pkey).expect("Failed to create verifier");
    verifier.update(data.as_bytes()).expect("Failed to write data to verifier");

    verifier.verify(&signature).is_ok()
}

