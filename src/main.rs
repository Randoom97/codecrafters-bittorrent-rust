mod bdecoder;
mod buffered_stream;

use buffered_stream::BufferedStream;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_string = (&args[2]).clone();
        let mut buf_stream = BufferedStream::new(encoded_string.as_bytes());
        let decoded_value = bdecoder::decode(&mut buf_stream);
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
