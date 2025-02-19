use std::{collections::HashMap, io::Write, net::TcpStream};

use bytes::Buf;

use crate::{
    bformat::{bdecoder, btype::BType},
    buffered_stream::BufferedStream,
    torrent_info::TorrentInfo,
};

pub async fn discovery(torrent_info: &TorrentInfo) -> BType {
    let params = HashMap::from([
        ("peer_id", "1234567890abcdefghij".to_owned()),
        ("port", "6881".to_owned()),
        ("uploaded", "0".to_owned()),
        ("downloaded", "0".to_owned()),
        ("left", format!("{}", torrent_info.length)),
        ("compact", "1".to_owned()),
    ]);

    let mut url = reqwest::Url::parse_with_params(torrent_info.url.as_str(), params).unwrap();
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
    bdecoder::decode(&mut response_reader)
}

pub fn handshake(torrent_info: &TorrentInfo, host: &String) -> Vec<u8> {
    let mut handshake_message: Vec<u8> = Vec::new();
    handshake_message.push(19);
    handshake_message.append(&mut "BitTorrent protocol".bytes().collect());
    handshake_message.append(&mut vec![0, 0, 0, 0, 0, 0, 0, 0]);
    handshake_message.append(&mut torrent_info.info_hash.clone());
    handshake_message.append(&mut "1234567890abcdefghij".bytes().collect());

    let mut stream = TcpStream::connect(host).unwrap();
    stream.write_all(&handshake_message).unwrap();
    stream.flush().unwrap();

    let mut stream_reader = BufferedStream::new(stream);
    assert_eq!(
        handshake_message[..20],
        stream_reader.read_n_bytes(20).unwrap()
    );
    stream_reader.read_n_bytes(8).unwrap();
    assert_eq!(
        torrent_info.info_hash,
        stream_reader.read_n_bytes(20).unwrap()
    );
    stream_reader.read_n_bytes(20).unwrap()
}
