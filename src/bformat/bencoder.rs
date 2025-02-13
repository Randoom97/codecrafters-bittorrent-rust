use std::collections::HashMap;

use super::btype::BType;

pub fn encode(value: &BType) -> Vec<u8> {
    match value {
        BType::Bytes(bytes) => encode_bytes(bytes),
        BType::Number(number) => encode_number(number),
        BType::List(list) => encode_list(list),
        BType::Map(map) => encode_map(map),
    }
}

fn encode_bytes(bytes: &Vec<u8>) -> Vec<u8> {
    let mut encoded_bytes: Vec<u8> = format!("{}:", bytes.len()).bytes().collect();
    encoded_bytes.append(&mut bytes.clone());
    encoded_bytes
}

fn encode_number(number: &i128) -> Vec<u8> {
    format!("i{}e", number).bytes().collect()
}

fn encode_list(list: &Vec<Box<BType>>) -> Vec<u8> {
    let mut encoded_bytes: Vec<u8> = vec![b'l'];
    for item in list {
        encoded_bytes.append(&mut encode(item));
    }
    encoded_bytes.push(b'e');
    encoded_bytes
}

fn encode_map(map: &HashMap<String, Box<BType>>) -> Vec<u8> {
    let mut encoded_bytes: Vec<u8> = vec![b'd'];
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort_unstable();
    for key in keys {
        let value = map.get(key).unwrap();
        encoded_bytes.append(&mut encode_bytes(&key.bytes().collect()));
        encoded_bytes.append(&mut encode(value));
    }
    encoded_bytes.push(b'e');
    encoded_bytes
}
