use std::{collections::VecDeque, io::Read};

pub struct BufferedStream<T: Read> {
    reader: T,
    buffer: VecDeque<u8>,
}

impl<T: Read> BufferedStream<T> {
    pub fn new(reader: T) -> BufferedStream<T> {
        BufferedStream {
            reader,
            buffer: VecDeque::new(),
        }
    }

    pub fn read_n_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        if self.buffer.len() > n {
            return Some(self.buffer.drain(..n).collect());
        }
        let mut result: Vec<u8> = self.buffer.drain(..).collect();
        result.append(
            self.read_n_bytes_unbuffered(n - result.len())
                .as_mut()
                .unwrap(),
        );
        return Some(result);
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if self.buffer.len() == 0 {
            return self.read_byte_unbuffered();
        }
        return self.buffer.pop_front();
    }

    pub fn peek_byte(&mut self) -> Option<u8> {
        if self.buffer.len() == 0 {
            let byte = self.read_byte_unbuffered().unwrap();
            self.buffer.push_back(byte);
        }
        return self.buffer.front().cloned();
    }

    pub fn read_until(&mut self, byte_match: u8) -> Option<Vec<u8>> {
        let mut result: Vec<u8> = Vec::new();
        loop {
            let byte = self.read_byte();
            if byte.is_none() {
                return None;
            }
            if byte.unwrap() == byte_match {
                return Some(result);
            }
            result.push(byte.unwrap());
        }
    }

    fn read_n_bytes_unbuffered(&mut self, n: usize) -> Option<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.reader.read_exact(&mut buf).unwrap();
        return Some(buf);
    }

    fn read_byte_unbuffered(&mut self) -> Option<u8> {
        return self.read_n_bytes_unbuffered(1).unwrap().get(0).cloned();
    }
}
