use std::{collections::HashMap, fs::File};

use sha1::{Digest, Sha1};

use crate::{
    bformat::{bdecoder, bencoder},
    buffered_stream::BufferedStream,
};

pub struct TorrentInfo {
    pub url: String,
    pub length: usize,
    pub info_hash: Vec<u8>,
    pub piece_length: usize,
    pub piece_hashes: Vec<Vec<u8>>,
}

impl TorrentInfo {
    pub fn from_file(filepath: &String) -> TorrentInfo {
        let file = File::open(filepath).unwrap();
        let mut buf_stream = BufferedStream::new(file);
        let object = bdecoder::decode_map(&mut buf_stream);

        let url = object.get("announce").unwrap().to_string();

        let info_btype = object.get("info").unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bencoder::encode(info_btype));
        let info_hash: Vec<u8> = hasher.finalize().iter().cloned().collect();

        let info = info_btype.as_map().unwrap();
        let length = usize::try_from(*info.get("length").unwrap().as_number().unwrap()).unwrap();

        let piece_length =
            usize::try_from(*info.get("piece length").unwrap().as_number().unwrap()).unwrap();

        let mut piece_hashes = Vec::new();
        let pieces = info.get("pieces").unwrap().as_bytes().unwrap();
        let mut start = 0;
        while start < pieces.len() {
            piece_hashes.push(pieces[start..start + 20].iter().cloned().collect());
            start += 20;
        }

        TorrentInfo {
            url,
            length,
            info_hash,
            piece_length,
            piece_hashes,
        }
    }

    pub fn from_link(link: &String) -> Result<TorrentInfo, String> {
        let parts: Vec<&str> = link[8..].split('&').collect();

        let mut info_hash_option: Option<Vec<u8>> = None;
        let mut file_name_option: Option<String> = None;
        let mut url_option: Option<String> = None;
        for part in parts {
            if part.starts_with("xt=urn:btih:") {
                info_hash_option = hex::decode(&part[12..]).ok();
            }
            if part.starts_with("dn=") {
                file_name_option = Some(part[3..].to_owned());
            }
            if part.starts_with("tr=") {
                // deserializes into a map of {"url", ""} for some reason, have to do the awful mapping to turn it back into a string
                url_option =
                    serde_urlencoded::from_str::<HashMap<String, String>>(&part[3..].to_owned())
                        .ok()
                        .map(|u| u.keys().collect::<Vec<&String>>().pop().unwrap().to_owned());
            }
        }

        if info_hash_option.is_none() {
            return Err("Magnet link is missing info hash!".to_owned());
        }
        if file_name_option.is_none() {
            return Err("Magnet link is missing file name!".to_owned());
        }
        if url_option.is_none() {
            return Err("Magnet link is missing tracker url!".to_owned());
        }
        return Ok(TorrentInfo {
            url: url_option.unwrap(),
            length: 999, // needs to be greater than 0 for handshake
            info_hash: info_hash_option.unwrap(),
            piece_length: 0,
            piece_hashes: Vec::new(),
            // file_name: file_name_option.unwrap(),
        });
    }
}
