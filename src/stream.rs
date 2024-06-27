use std::{
    io::{Error, Read, Write},
    mem::ManuallyDrop,
    net::TcpStream,
};

use crate::frame::{Frame, Opcode};

pub struct WsStream {
    frame: Frame,
    pub stream: ManuallyDrop<TcpStream>,
    buffer: Vec<u8>,
    cursor: usize,
}

impl WsStream {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            frame: Frame::new(true, Opcode::Text, None, 0),
            stream: ManuallyDrop::new(stream),
            cursor: 0,
            buffer: vec![],
        }
    }

    fn sext(&mut self, message: &str, isfinal: bool, mask: Option<[u8; 4]>) -> Result<(), Error> {
        self.frame = Frame::new(
            isfinal,
            match isfinal {
                true => Opcode::Text,
                false => Opcode::Continuation,
            },
            mask,
            message.len(),
        );
        self.write_frame()?;
        self.write_all(message.as_bytes())
    }

    pub fn bin(&mut self, message: Vec<u8>, isfinal: bool, mask: Option<[u8; 4]>) -> Result<(), Error> {
        self.frame = Frame::new(
            isfinal,
            match isfinal {
                true => Opcode::Text,
                false => Opcode::Continuation,
            },
            mask,
            message.len(),
        );
        self.write_frame()?;
        self.write_all(&message)
    }

    pub fn text_fragment(&mut self, message: &str, mask: Option<[u8; 4]>) -> Result<(), Error> {
        self.sext(message, false, mask)
    }
    pub fn text(&mut self, message: &str, mask: Option<[u8; 4]>) -> Result<(), Error> {
        self.sext(message, true, mask)
    }
    pub fn binary_fragment(&mut self, message: Vec<u8>, mask: Option<[u8; 4]>) -> Result<(), Error> {
        self.bin(message, false, mask)
    }
    pub fn binary(&mut self, message: Vec<u8>, mask: Option<[u8; 4]>) -> Result<(), Error> {
        self.bin(message, true, mask)
    }

    pub fn close(&mut self, mask: Option<[u8; 4]>, status: u16) -> Result<(), Error> {
        self. frame = Frame::new(true, Opcode::Close, mask, 2);
        self.write_frame()?;
        self.write_all(&status.to_be_bytes())
    }

    pub fn bye(&mut self, mask: Option<[u8; 4]>) -> Result<(), Error> {
        let mut buf: Vec<u8> = vec![0, 0];
        self.read_exact(&mut buf).expect("failed to read to end");
        self.frame = Frame::new(true, Opcode::Close, mask, 2);
        self.write_frame()?;
        self.write_all(&buf)
    }

    pub fn ping(&mut self, message: &str, mask: Option<[u8; 4]>) ->Result<(), Error>  {
        self.frame = Frame::new(true, Opcode::Ping, mask, message.len());
        self.write_frame()?;
        self.write_all(message.as_bytes())
    }

    pub fn pong(&mut self, mask: Option<[u8; 4]>) -> Result<(), Error> {
        let mut buf: Vec<u8> = vec![];
        let n = self.read_to_end(&mut buf).expect("failed to read to end");
        self.frame = Frame::new(true, Opcode::Pong, mask, n);
        self.write_frame()?;
        self.write_all(&buf)
    }

    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, Error> {
        self.stream.peer_addr()
    }

    pub fn read_frame(&mut self) {
        self.frame = Frame::read_frame(&self.stream);
    }

    pub fn opcode(&self)-> Opcode {
        self.frame.opcode
    }

    fn write_frame(&mut self) -> Result<(), Error> {
        let blob = self.frame.to_blob();
        self.write_all(&blob)
    }
}

impl Read for WsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.cursor == self.frame.payload_length {
            self.cursor = 0;
            return Ok(0);
        }
        let n = self.stream.read(buf)?;
        match self.frame.mask {
            Some(mask) => {
                for c in 0..n {
                    buf[c] = buf[c] ^ mask[self.cursor % 4];
                    self.cursor += 1;
                }
            }
            None => {}
        }
        Ok(n)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let mut cursor = 0;
        loop {
            let n = self.read(&mut buf[cursor..])?;
            cursor += n;
            if n == 0 {
                break Ok(cursor);
            }
        }
    }
}

impl Write for WsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        //if self.cursor + buf.len() > self.frame.payload_length {
        //    return Err(Error::new(
        //        std::io::ErrorKind::InvalidInput,
        //        "too long input",
        //    ));
        //}
        match self.frame.mask {
            Some(mask) => {
                let _ = buf
                    .bytes()
                    .map(|b| self.buffer.push(b.unwrap() ^ mask[self.cursor % 4]));
                self.flush().unwrap()
            }
            None => {
                self.stream
                    .write_all(buf)
                    .expect("failed to write to stream");
            }
        }
        self.cursor += buf.len();

        if self.cursor == self.frame.payload_length {
            self.cursor = 0;
            self.frame.payload_length = 0;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let result = self.stream.write_all(&self.buffer);
        self.buffer.clear();
        result
    }
}
