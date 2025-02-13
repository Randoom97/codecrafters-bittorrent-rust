mod bformat;
mod buffered_stream;

use bformat::{bdecoder, bencoder};
use buffered_stream::BufferedStream;
use sha1::{Digest, Sha1};
use std::{env, fs::File};

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    match command.as_str() {
        "decode" => {
            let encoded_string = (&args[2]).clone();
            let mut buf_stream = BufferedStream::new(encoded_string.as_bytes());
            let decoded_value = bdecoder::decode(&mut buf_stream);
            println!("{}", decoded_value.to_json_value().to_string());
        }
        "info" => {
            let file = File::open(&args[2]).unwrap();
            let mut buf_stream = BufferedStream::new(file);
            let object = bdecoder::decode_map(&mut buf_stream);
            let url = object.get("announce").unwrap().to_string();
            let info_btype = object.get("info").unwrap();
            let info = info_btype.as_map().unwrap();
            let length = info.get("length").unwrap().as_number().unwrap();
            let mut hasher = Sha1::new();
            hasher.update(bencoder::encode(info_btype));
            println!(
                "Tracker URL: {}\nLength: {}\nInfo Hash: {:#x}",
                url,
                length,
                hasher.finalize()
            );
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}
