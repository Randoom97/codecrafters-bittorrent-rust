use std::{
    cmp::min,
    collections::{HashMap, VecDeque},
    io::{Read, Write},
};

use bytes::Buf;
use sha1::{Digest, Sha1};

use crate::{
    bformat::{bdecoder, bencoder, btype::BType},
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

// return value is in the form of (Peer ID, peer reserved bytes)
pub fn handshake<T: Read>(
    torrent_info: &TorrentInfo,
    writer: &mut impl Write,
    reader: &mut BufferedStream<T>,
    reserved_bytes: Option<[u8; 8]>,
) -> (Vec<u8>, Vec<u8>) {
    let mut handshake_message: Vec<u8> = Vec::new();
    handshake_message.push(19);
    handshake_message.append(&mut "BitTorrent protocol".bytes().collect());
    handshake_message.append(&mut Vec::from(
        reserved_bytes.unwrap_or([0, 0, 0, 0, 0, 0, 0, 0]),
    ));
    handshake_message.append(&mut torrent_info.info_hash.clone());
    handshake_message.append(&mut "1234567890abcdefghij".bytes().collect());

    writer.write_all(&handshake_message).unwrap();
    writer.flush().unwrap();

    assert_eq!(handshake_message[..20], reader.read_n_bytes(20).unwrap());
    let peer_reserved_bytes = reader.read_n_bytes(8).unwrap();
    assert_eq!(torrent_info.info_hash, reader.read_n_bytes(20).unwrap());
    let peer_id = reader.read_n_bytes(20).unwrap();

    (peer_id, peer_reserved_bytes)
}

pub fn extension_handshake<T: Read>(writer: &mut impl Write, reader: &mut BufferedStream<T>) -> u8 {
    // wait for bitfield
    let _ = read_peer_message(reader);

    let mut message = bencoder::encode(&BType::Map(HashMap::from([(
        "m".to_owned(),
        Box::new(BType::Map(HashMap::from([(
            "ut_metadata".to_owned(),
            Box::new(BType::Number(1)),
        )]))),
    )])));
    writer
        .write_all(&mut to_vec((message.len() + 2) as u32))
        .unwrap();
    writer.write_all(&mut vec![20, 0]).unwrap();
    writer.write_all(&mut message).unwrap();
    writer.flush().unwrap();

    let response = read_peer_message(reader);
    assert_eq!(response[..2], vec![20, 0][..2], "message id's didn't match");
    let btype = bdecoder::decode(&mut BufferedStream::new(response[2..].reader()));
    let map = btype.as_map().unwrap();
    assert!(
        map.contains_key("m"),
        "response dictionary didn't have an 'm' entry"
    );
    let extension_map = map.get("m").unwrap().as_map().unwrap();
    assert!(
        extension_map.contains_key("ut_metadata"),
        "extension dictionary didn't contain an entry for 'ut_metadata'"
    );
    *extension_map
        .get("ut_metadata")
        .unwrap()
        .as_number()
        .unwrap() as u8
}

pub fn request_metadata<T: Read>(
    partial_torrent_info: &TorrentInfo,
    metadata_id: u8,
    writer: &mut impl Write,
    reader: &mut BufferedStream<T>,
) -> TorrentInfo {
    let message = bencoder::encode(&BType::Map(HashMap::from([
        ("msg_type".to_owned(), Box::new(BType::Number(0))),
        ("piece".to_owned(), Box::new(BType::Number(0))),
    ])));

    writer
        .write_all(&mut to_vec((message.len() + 2) as u32))
        .unwrap();
    writer.write_all(&mut [20, metadata_id]).unwrap();
    writer.write_all(&message).unwrap();
    writer.flush().unwrap();

    let response = read_peer_message(reader);
    // not using metadata_id as second variable here because we sent 1 as our metadata id during handshake
    assert_eq!(response[..2], vec![20, 1]);

    let mut response_stream = BufferedStream::new(response[2..].reader());
    let btype = bdecoder::decode(&mut response_stream);
    let map = btype.as_map().unwrap();
    assert!(map.contains_key("msg_type"));
    assert_eq!(map.get("msg_type").unwrap().as_number(), Some(&1));
    assert!(map.contains_key("piece"));
    assert_eq!(map.get("piece").unwrap().as_number(), Some(&0));

    TorrentInfo::from_metadata(partial_torrent_info, &mut response_stream)
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

    // vec of (begin, length)
    let mut blocks_needed: VecDeque<(u32, u32)> = VecDeque::new();
    let piece_size: usize = min(
        torrent_info.piece_length,
        torrent_info.length - (torrent_info.piece_length * piece_index),
    );
    let mut block_count = piece_size / 0x4000;
    if piece_size % 0x4000 != 0 {
        block_count += 1;
    }
    for i in 0..block_count {
        blocks_needed.push_back((
            (i * 0x4000) as u32,
            if piece_size % 0x4000 != 0 && i == block_count - 1 {
                (piece_size % 0x4000) as u32
            } else {
                0x4000
            },
        ));
    }

    // request & receive blocks
    let mut piece = Vec::new();
    let mut pending: VecDeque<(u32, u32)> = VecDeque::new();
    while blocks_needed.len() > 0 || pending.len() > 0 {
        while pending.len() < 5 && blocks_needed.len() > 0 {
            // request up to 5 items
            let mut request = to_vec(13);
            request.push(6);
            request.append(&mut to_vec(piece_index as u32));

            let (begin, length) = blocks_needed.pop_front().unwrap();
            request.append(&mut to_vec(begin));
            request.append(&mut to_vec(length));

            writer.write_all(&mut request).unwrap();

            pending.push_back((begin, length));
        }
        writer.flush().unwrap();

        loop {
            let message = read_peer_message(reader);
            if message.len() == 0 || message[0] != 7 {
                continue;
            }

            let (begin, length) = pending.pop_front().unwrap();

            assert_eq!(to_vec(piece_index as u32), message[1..5]);
            assert_eq!(to_vec(begin as u32), message[5..9]);
            let mut data: Vec<u8> = (&message[9..]).iter().cloned().collect();
            assert_eq!(length as usize, data.len());
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
