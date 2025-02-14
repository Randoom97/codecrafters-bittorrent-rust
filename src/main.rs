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
            let mut info_string = String::new();

            let file = File::open(&args[2]).unwrap();
            let mut buf_stream = BufferedStream::new(file);
            let object = bdecoder::decode_map(&mut buf_stream);

            let url = object.get("announce").unwrap().to_string();
            info_string.push_str(format!("Tracker URL: {url}").as_str());

            let info_btype = object.get("info").unwrap();
            let info = info_btype.as_map().unwrap();
            let length = info.get("length").unwrap().as_number().unwrap();
            info_string.push_str(format!("\nLength: {length}").as_str());

            let mut hasher = Sha1::new();
            hasher.update(bencoder::encode(info_btype));
            info_string.push_str(format!("\nInfo Hash: {:#x}", hasher.finalize()).as_str());

            let piece_length = info.get("piece length").unwrap().as_number().unwrap();
            info_string.push_str(format!("\nPiece Length: {piece_length}").as_str());

            info_string.push_str("\nPiece Hashes:");
            let piece_hashes = info.get("pieces").unwrap().as_bytes().unwrap();
            let mut start = 0;
            while start < piece_hashes.len() {
                let hash = hex::encode(&piece_hashes[start..start + 20]);
                info_string.push_str(format!("\n{hash}",).as_str());
                start += 20;
            }

            println!("{info_string}");
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}
