use std::{collections::HashMap, io::Read};

use super::btype::BType;
use crate::buffered_stream::BufferedStream;

// todo don't save everything in serde_json::Value. Need to store some strings as Vec<u8> if they fail to convert from utf8
pub fn decode<T: Read>(buf_stream: &mut BufferedStream<T>) -> BType {
    let first_byte = buf_stream.peek_byte().unwrap();
    if first_byte.is_ascii_digit() {
        BType::Bytes(decode_bytes(buf_stream))
    } else if first_byte == b'i' {
        BType::Number(decode_number(buf_stream))
    } else if first_byte == b'l' {
        BType::List(decode_list(buf_stream))
    } else if first_byte == b'd' {
        BType::Map(decode_map(buf_stream))
    } else {
        panic!(
            "Unable to determine bencode type from first byte: {}",
            first_byte
        );
    }
}

fn decode_bytes<T: Read>(buf_stream: &mut BufferedStream<T>) -> Vec<u8> {
    let length = String::from_utf8(buf_stream.read_until(b':').unwrap())
        .unwrap()
        .parse::<usize>()
        .unwrap();
    return buf_stream.read_n_bytes(length).unwrap();
}

fn decode_number<T: Read>(buf_stream: &mut BufferedStream<T>) -> i128 {
    buf_stream.read_byte(); // skip the 'i'
    String::from_utf8(buf_stream.read_until(b'e').unwrap())
        .unwrap()
        .parse::<i128>()
        .unwrap()
}

fn decode_list<T: Read>(buf_stream: &mut BufferedStream<T>) -> Vec<Box<BType>> {
    buf_stream.read_byte(); // skip the 'l'
    let mut values = Vec::new();
    while buf_stream.peek_byte().unwrap() != b'e' {
        values.push(Box::new(decode(buf_stream)));
    }
    buf_stream.read_byte(); // skip the trailing 'e'
    return values;
}

pub fn decode_map<T: Read>(buf_stream: &mut BufferedStream<T>) -> HashMap<String, Box<BType>> {
    buf_stream.read_byte(); // skip the 'd'
    let mut map = HashMap::new();
    while buf_stream.peek_byte().unwrap() != b'e' {
        let key = String::from_utf8(decode_bytes(buf_stream)).unwrap();
        let value = decode(buf_stream);
        map.insert(key, Box::new(value));
    }
    buf_stream.read_byte(); // skip the trailing 'e'
    return map;
}
