mod bformat;
mod buffered_stream;
mod torrent_info;
mod torrent_protocol;

use bformat::bdecoder;
use buffered_stream::BufferedStream;
use std::{env, fs::File, io::Write, net::TcpStream};
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
            print_torrent_info(&torrent_info);
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
            let mut writer = TcpStream::connect(&args[3]).unwrap();
            let mut reader = BufferedStream::new(writer.try_clone().unwrap());
            let (peer_id, _) =
                torrent_protocol::handshake(&torrent_info, &mut writer, &mut reader, None);
            println!("Peer ID: {}", hex::encode(peer_id));
        }
        "download_piece" => {
            let torrent_info = TorrentInfo::from_file(&args[4]);
            let response_btype = torrent_protocol::discovery(&torrent_info).await;
            let response = response_btype.as_map().unwrap();
            let peer = &human_readable_peers(response.get("peers").unwrap().as_bytes().unwrap())[0];

            let mut writer = TcpStream::connect(peer).unwrap();
            let mut reader = BufferedStream::new(writer.try_clone().unwrap());
            torrent_protocol::handshake(&torrent_info, &mut writer, &mut reader, None);

            torrent_protocol::send_interested(&mut writer, &mut reader);

            let data = torrent_protocol::download_piece(
                &torrent_info,
                (&args[5]).parse::<usize>().unwrap().to_owned(),
                &mut writer,
                &mut reader,
            )
            .unwrap();
            let mut file = File::create(&args[3]).unwrap();
            file.write_all(&data).unwrap();
            file.flush().unwrap();
        }
        "download" => {
            let torrent_info = TorrentInfo::from_file(&args[4]);
            let response_btype = torrent_protocol::discovery(&torrent_info).await;
            let response = response_btype.as_map().unwrap();
            let peer = &human_readable_peers(response.get("peers").unwrap().as_bytes().unwrap())[0];

            let mut writer = TcpStream::connect(peer).unwrap();
            let mut reader = BufferedStream::new(writer.try_clone().unwrap());
            torrent_protocol::handshake(&torrent_info, &mut writer, &mut reader, None);

            torrent_protocol::send_interested(&mut writer, &mut reader);

            let mut file = File::create(&args[3]).unwrap();
            for i in 0..torrent_info.piece_hashes.len() {
                let data =
                    torrent_protocol::download_piece(&torrent_info, i, &mut writer, &mut reader)
                        .unwrap();
                file.write_all(&data).unwrap();
            }
            file.flush().unwrap();
        }
        "magnet_parse" => {
            let torrent_info_result = TorrentInfo::from_link(&args[2]);
            if torrent_info_result.is_err() {
                panic!("{}", torrent_info_result.err().unwrap());
            }
            let torrent_info = torrent_info_result.unwrap();

            println!(
                "Tracker URL: {}\nInfo Hash: {}",
                torrent_info.url,
                hex::encode(torrent_info.info_hash)
            );
        }
        "magnet_handshake" => {
            let torrent_info_result = TorrentInfo::from_link(&args[2]);
            if torrent_info_result.is_err() {
                panic!("{}", torrent_info_result.err().unwrap());
            }
            let torrent_info = torrent_info_result.unwrap();

            let response_btype = torrent_protocol::discovery(&torrent_info).await;
            let response = response_btype.as_map().unwrap();
            let peer = &human_readable_peers(response.get("peers").unwrap().as_bytes().unwrap())[0];

            let mut writer = TcpStream::connect(peer).unwrap();
            let mut reader = BufferedStream::new(writer.try_clone().unwrap());
            let (peer_id, reserved_bytes) = torrent_protocol::handshake(
                &torrent_info,
                &mut writer,
                &mut reader,
                Some([0, 0, 0, 0, 0, 0x10, 0, 0]),
            );
            println!("Peer ID: {}", hex::encode(peer_id));

            if reserved_bytes[5] & 0x10 != 0 {
                let metadata_id = torrent_protocol::extension_handshake(&mut writer, &mut reader);
                println!("Peer Metadata Extension ID: {metadata_id}");
            }
        }
        "magnet_info" => {
            let partial_torrent_info_result = TorrentInfo::from_link(&args[2]);
            if partial_torrent_info_result.is_err() {
                panic!("{}", partial_torrent_info_result.err().unwrap());
            }
            let partial_torrent_info = partial_torrent_info_result.unwrap();

            let response_btype = torrent_protocol::discovery(&partial_torrent_info).await;
            let response = response_btype.as_map().unwrap();
            let peer = &human_readable_peers(response.get("peers").unwrap().as_bytes().unwrap())[0];

            let mut writer = TcpStream::connect(peer).unwrap();
            let mut reader = BufferedStream::new(writer.try_clone().unwrap());
            let (_, reserved_bytes) = torrent_protocol::handshake(
                &partial_torrent_info,
                &mut writer,
                &mut reader,
                Some([0, 0, 0, 0, 0, 0x10, 0, 0]),
            );

            if reserved_bytes[5] & 0x10 == 0 {
                panic!("expected to have a peer that supports extensions");
            }

            let metadata_id = torrent_protocol::extension_handshake(&mut writer, &mut reader);
            let torrent_info = torrent_protocol::request_metadata(
                &partial_torrent_info,
                metadata_id,
                &mut writer,
                &mut reader,
            );
            print_torrent_info(&torrent_info);
        }
        "magnet_download_piece" => {
            let partial_torrent_info_result = TorrentInfo::from_link(&args[4]);
            if partial_torrent_info_result.is_err() {
                panic!("{}", partial_torrent_info_result.err().unwrap());
            }
            let partial_torrent_info = partial_torrent_info_result.unwrap();

            let response_btype = torrent_protocol::discovery(&partial_torrent_info).await;
            let response = response_btype.as_map().unwrap();
            let peer = &human_readable_peers(response.get("peers").unwrap().as_bytes().unwrap())[0];

            let mut writer = TcpStream::connect(peer).unwrap();
            let mut reader = BufferedStream::new(writer.try_clone().unwrap());
            let (_, reserved_bytes) = torrent_protocol::handshake(
                &partial_torrent_info,
                &mut writer,
                &mut reader,
                Some([0, 0, 0, 0, 0, 0x10, 0, 0]),
            );

            if reserved_bytes[5] & 0x10 == 0 {
                panic!("expected to have a peer that supports extensions");
            }

            let metadata_id = torrent_protocol::extension_handshake(&mut writer, &mut reader);
            let torrent_info = torrent_protocol::request_metadata(
                &partial_torrent_info,
                metadata_id,
                &mut writer,
                &mut reader,
            );

            torrent_protocol::send_interested(&mut writer, &mut reader);

            let data = torrent_protocol::download_piece(
                &torrent_info,
                (&args[5]).parse::<usize>().unwrap().to_owned(),
                &mut writer,
                &mut reader,
            )
            .unwrap();
            let mut file = File::create(&args[3]).unwrap();
            file.write_all(&data).unwrap();
            file.flush().unwrap();
        }
        "magnet_download" => {
            let partial_torrent_info_result = TorrentInfo::from_link(&args[4]);
            if partial_torrent_info_result.is_err() {
                panic!("{}", partial_torrent_info_result.err().unwrap());
            }
            let partial_torrent_info = partial_torrent_info_result.unwrap();

            let response_btype = torrent_protocol::discovery(&partial_torrent_info).await;
            let response = response_btype.as_map().unwrap();
            let peer = &human_readable_peers(response.get("peers").unwrap().as_bytes().unwrap())[0];

            let mut writer = TcpStream::connect(peer).unwrap();
            let mut reader = BufferedStream::new(writer.try_clone().unwrap());
            let (_, reserved_bytes) = torrent_protocol::handshake(
                &partial_torrent_info,
                &mut writer,
                &mut reader,
                Some([0, 0, 0, 0, 0, 0x10, 0, 0]),
            );

            if reserved_bytes[5] & 0x10 == 0 {
                panic!("expected to have a peer that supports extensions");
            }

            let metadata_id = torrent_protocol::extension_handshake(&mut writer, &mut reader);
            let torrent_info = torrent_protocol::request_metadata(
                &partial_torrent_info,
                metadata_id,
                &mut writer,
                &mut reader,
            );

            torrent_protocol::send_interested(&mut writer, &mut reader);

            let mut file = File::create(&args[3]).unwrap();
            for i in 0..torrent_info.piece_hashes.len() {
                let data =
                    torrent_protocol::download_piece(&torrent_info, i, &mut writer, &mut reader)
                        .unwrap();
                file.write_all(&data).unwrap();
            }
            file.flush().unwrap();
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}

fn print_torrent_info(torrent_info: &TorrentInfo) {
    let mut info_string = format!(
        "Tracker URL: {}\nLength: {}\nInfo Hash: {}\nPiece Length: {}\nPiece Hashes:",
        torrent_info.url,
        torrent_info.length,
        hex::encode(&torrent_info.info_hash),
        torrent_info.piece_length,
    );
    for hash in &torrent_info.piece_hashes {
        info_string.push_str(format!("\n{}", hex::encode(hash)).as_str());
    }
    println!("{info_string}");
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
