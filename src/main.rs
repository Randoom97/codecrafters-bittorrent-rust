mod bformat;
mod buffered_stream;
mod torrent_info;
mod torrent_protocol;

use bformat::bdecoder;
use buffered_stream::BufferedStream;
use std::env;
use torrent_info::TorrentInfo;

#[tokio::main]
async fn main() {
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
            let torrent_info = TorrentInfo::from_file(&args[2]);
            let mut info_string = format!(
                "Tracker URL: {}\nLength: {}\nInfo Hash: {}\nPiece Length: {}\nPiece Hashes:",
                torrent_info.url,
                torrent_info.length,
                hex::encode(torrent_info.info_hash),
                torrent_info.piece_length,
            );
            for hash in torrent_info.piece_hashes {
                info_string.push_str(format!("\n{}", hex::encode(hash)).as_str());
            }
            println!("{info_string}");
        }
        "peers" => {
            let torrent_info = TorrentInfo::from_file(&args[2]);

            let response_btype = torrent_protocol::discovery(&torrent_info).await;
            let response = response_btype.as_map().unwrap();
            let peers = human_readable_peers(response.get("peers").unwrap().as_bytes().unwrap());

            let mut peers_string = String::new();
            for peer in peers {
                peers_string.push_str(format!("{peer}\n").as_str());
            }
            peers_string.pop();
            println!("{}", peers_string);
        }
        "handshake" => {
            let torrent_info = TorrentInfo::from_file(&args[2]);
            let peer_id = torrent_protocol::handshake(&torrent_info, &args[3]);
            println!("Peer ID: {}", hex::encode(peer_id));
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}

fn human_readable_peers(peer_bytes: &Vec<u8>) -> Vec<String> {
    let mut peer_strings = Vec::new();
    let mut start = 0;
    while start < peer_bytes.len() {
        let peer_slice = &peer_bytes[start..start + 6];
        peer_strings.push(format!(
            "{}.{}.{}.{}:{}",
            peer_slice[0],
            peer_slice[1],
            peer_slice[2],
            peer_slice[3],
            ((peer_slice[4] as u16) << 8) + peer_slice[5] as u16
        ));
        start += 6;
    }
    peer_strings
}
