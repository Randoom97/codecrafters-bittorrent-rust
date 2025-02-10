use std::io::Read;

use serde_json::Number;

use crate::buffered_stream::BufferedStream;

pub fn decode<T: Read>(buf_stream: &mut BufferedStream<T>) -> serde_json::Value {
    let first_byte = buf_stream.peek_byte().unwrap();
    if first_byte.is_ascii_digit() {
        return serde_json::Value::String(decode_string(buf_stream));
    } else if first_byte == b'i' {
        buf_stream.read_byte(); // skip the 'i'
        return serde_json::Value::Number(Number::from_i128(decode_number(buf_stream)).unwrap());
    } else if first_byte == b'l' {
        buf_stream.read_byte(); // skip the 'l'
        let mut values: Vec<serde_json::Value> = Vec::new();
        while buf_stream.peek_byte().unwrap() != b'e' {
            values.push(decode(buf_stream));
        }
        buf_stream.read_byte(); // skip the trailing 'e'
        return serde_json::Value::Array(values);
    } else {
        panic!(
            "Unable to determine bencode type from first byte: {}",
            first_byte
        );
    }
}

fn decode_string<T: Read>(buf_stream: &mut BufferedStream<T>) -> String {
    let length = String::from_utf8(buf_stream.read_until(b':').unwrap())
        .unwrap()
        .parse::<usize>()
        .unwrap();
    return String::from_utf8(buf_stream.read_n_bytes(length).unwrap()).unwrap();
}

fn decode_number<T: Read>(buf_stream: &mut BufferedStream<T>) -> i128 {
    String::from_utf8(buf_stream.read_until(b'e').unwrap())
        .unwrap()
        .parse::<i128>()
        .unwrap()
}
