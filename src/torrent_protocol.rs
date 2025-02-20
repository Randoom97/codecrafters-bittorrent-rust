use std::{
    cmp::min,
    collections::HashMap,
    io::{Read, Write},
};

use bytes::Buf;
use sha1::{Digest, Sha1};

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

pub fn handshake<T: Read>(
    torrent_info: &TorrentInfo,
    writer: &mut impl Write,
    reader: &mut BufferedStream<T>,
) -> Vec<u8> {
    let mut handshake_message: Vec<u8> = Vec::new();
    handshake_message.push(19);
    handshake_message.append(&mut "BitTorrent protocol".bytes().collect());
    handshake_message.append(&mut vec![0, 0, 0, 0, 0, 0, 0, 0]);
    handshake_message.append(&mut torrent_info.info_hash.clone());
    handshake_message.append(&mut "1234567890abcdefghij".bytes().collect());

    writer.write_all(&handshake_message).unwrap();
    writer.flush().unwrap();

    assert_eq!(handshake_message[..20], reader.read_n_bytes(20).unwrap());
    reader.read_n_bytes(8).unwrap();
    assert_eq!(torrent_info.info_hash, reader.read_n_bytes(20).unwrap());
    let peer_id = reader.read_n_bytes(20).unwrap();

    peer_id
}

pub fn send_interested<T: Read>(writer: &mut impl Write, reader: &mut BufferedStream<T>) {
    writer.write_all(&mut vec![0, 0, 0, 1, 2]).unwrap();
    writer.flush().unwrap();

    // receive unchoke
    loop {
        let message = read_peer_message(reader);
        if message.len() > 0 && message[0] == 1 {
            break;
        }
    }
}

pub fn download_piece<T: Read>(
    torrent_info: &TorrentInfo,
    piece_index: usize,
    writer: &mut impl Write,
    reader: &mut BufferedStream<T>,
) -> Result<Vec<u8>, String> {
    if piece_index >= torrent_info.piece_hashes.len() {
        return Err(format!("Error: piece index {piece_index} out of range!"));
    }

    // request & receive blocks
    let mut piece = Vec::new();
    let mut data_left: usize = min(
        torrent_info.piece_length,
        torrent_info.length - (torrent_info.piece_length * piece_index),
    );
    while data_left > 0 {
        let mut request = to_vec(13);
        request.push(6);
        request.append(&mut to_vec(piece_index as u32));
        let begin = piece.len() as u32;
        request.append(&mut to_vec(begin));
        request.append(&mut to_vec(min(data_left, 0x4000) as u32));

        writer.write_all(&mut request).unwrap();
        writer.flush().unwrap();

        loop {
            let message = read_peer_message(reader);
            if message.len() == 0 || message[0] != 7 {
                continue;
            }

            assert_eq!(to_vec(piece_index as u32), message[1..5]);
            assert_eq!(to_vec(begin as u32), message[5..9]);
            let mut data: Vec<u8> = (&message[9..]).iter().cloned().collect();
            data_left -= data.len();
            piece.append(&mut data);
            break;
        }
    }

    // check hash
    let mut hasher = Sha1::new();
    hasher.update(&piece);
    let piece_hash: Vec<u8> = hasher.finalize().iter().cloned().collect();
    assert_eq!(torrent_info.piece_hashes[piece_index], piece_hash);

    return Ok(piece);
}

fn read_peer_message<T: Read>(reader: &mut BufferedStream<T>) -> Vec<u8> {
    let length = to_u32(reader.read_n_bytes(4).unwrap()).unwrap();
    if length == 0 {
        return Vec::new();
    }
    return reader.read_n_bytes(length as usize).unwrap();
}

fn to_u32(vec: Vec<u8>) -> Option<u32> {
    if vec.len() != 4 {
        return None;
    }
    Some(
        ((vec[0] as u32) << 24)
            | ((vec[1] as u32) << 16)
            | ((vec[2] as u32) << 8)
            | (vec[3] as u32),
    )
}

fn to_vec(number: u32) -> Vec<u8> {
    vec![
        (number >> 24) as u8,
        (number >> 16) as u8,
        (number >> 8) as u8,
        number as u8,
    ]
}
