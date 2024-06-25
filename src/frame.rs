use std::{io::Read, net::TcpStream};

#[derive(Debug, Copy, Clone)]
pub enum Opcode {
    Continuation,
    Text,
    Binary,
    Reserved,
    Close,
    Ping,
    Pong,
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Self {
        match value {
            0x0 => Self::Continuation,
            0x1 => Self::Text,
            0x2 => Self::Binary,
            0x8 => Self::Close,
            0x9 => Self::Ping,
            0xa => Self::Pong,
            _ => Self::Reserved,
        }
    }
}

impl Opcode {
    fn into_u8(self) -> u8 {
        match self {
            Self::Continuation => 0x0,
            Self::Text => 0x1,
            Self::Binary => 0x2,
            Self::Close => 0x8,
            Self::Ping => 0x9,
            Self::Pong => 0xa,
            Self::Reserved => 0xb,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub is_final: bool,
    pub opcode: Opcode,
    pub mask: Option<[u8; 4]>,
    pub payload_length: usize,
}

impl Frame {
    pub fn new(is_final: bool, opcode: Opcode, mask: Option<[u8; 4]>, payload_length: usize) -> Self {
        Self {
            is_final,
            opcode,
            mask,
            payload_length,
        }
    }

    pub fn to_blob(&self) -> Vec<u8> {
        let mut blob = vec![];
        let mut first_byte = 0x0u8;
        if self.is_final {
            first_byte |= 0x80;
        }
        first_byte |= self.opcode.into_u8();
        blob.push(first_byte);
        let mut second_byte = 0x0u8;
        if self.mask.is_some() {
            second_byte |= 0x80;
        }

        if self.payload_length < 126 {
            second_byte |= self.payload_length as u8;
            blob.push(second_byte);
        } else if self.payload_length <= u16::MAX as usize {
            second_byte |= 126;
            blob.push(second_byte);
            let _ = u16::to_be_bytes(self.payload_length as u16)
                .bytes()
                .map(|b| blob.push(b.unwrap()));
        } else {
            second_byte |= 127;
            blob.push(second_byte);
            let _ = u64::to_be_bytes(self.payload_length as u64)
                .bytes()
                .map(|b| blob.push(b.unwrap()));
        }

        self.mask.and_then(|mask| {
            let _ = mask.bytes().map(|b| blob.push(b.unwrap()));
            Some(())
        });

        blob
    }

    pub fn read_frame(mut stream: &TcpStream) -> Self {
        let mut buffer = [0u8; 2];
        stream
            .read_exact(&mut buffer)
            .expect("failed to read from stream");
        let n = buffer[0];
        let is_final = (n & 0x80) > 0;
        let opcode = n & 0xf;
        let n = buffer[1];
        let mask = (n & 0x80) > 0;
        let payload_len = n & 0x7f;
    
        let real_len = if payload_len < 126 {
            payload_len as usize
        } else if payload_len == 126 {
            stream
                .read_exact(&mut buffer)
                .expect("failed to read from stream");
            u16::from_be_bytes(buffer) as usize
        } else {
            let mut buffer = [0u8; 8];
            stream
                .read_exact(&mut buffer)
                .expect("failed to read from stream");
            u64::from_be_bytes(buffer) as usize
        };
    
        let mut buffer = [0u8; 4];
        if mask {
            stream
                .read_exact(&mut buffer)
                .expect("failed to read from stream");
        }
    
        Self {
            is_final,
            opcode: opcode.into(),
            mask: if mask { Some(buffer) } else { None },
            payload_length: real_len,
        }
    }
}

