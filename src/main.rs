mod bformat;
mod buffered_stream;
mod torrent_info;

use bformat::bdecoder;
use buffered_stream::BufferedStream;
use bytes::Buf;
use std::{collections::HashMap, env};
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

            let params = HashMap::from([
                ("peer_id", "1234567890abcdefghij".to_owned()),
                ("port", "6881".to_owned()),
                ("uploaded", "0".to_owned()),
                ("downloaded", "0".to_owned()),
                ("left", format!("{}", torrent_info.length)),
                ("compact", "1".to_owned()),
            ]);

            let mut url =
                reqwest::Url::parse_with_params(torrent_info.url.as_str(), params).unwrap();
            url.query_pairs_mut().append_pair("info_hash", unsafe {
                std::str::from_utf8_unchecked(&torrent_info.info_hash)
            });

            let mut response_reader = BufferedStream::new(
                reqwest::get(url)
                    .await
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap()
                    .reader(),
            );
            let response_btype = bdecoder::decode(&mut response_reader);
            let response = response_btype.as_map().unwrap();
            let peers = human_readable_peers(response.get("peers").unwrap().as_bytes().unwrap());

            let mut peers_string = String::new();
            for peer in peers {
                peers_string.push_str(format!("{peer}\n").as_str());
            }
            peers_string.pop();
            println!("{}", peers_string);
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
