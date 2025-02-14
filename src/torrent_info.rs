use std::fs::File;

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
}
