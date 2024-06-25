use std::{
    ffi::c_int,
    io::{BufRead, BufReader, Error, Read, Write},
    mem,
    net::{TcpListener, TcpStream},
    os::fd::{AsRawFd, FromRawFd},
    process::exit,
};

use mux::Mux;

mod base64;
mod mux;
mod sha1;

const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();
    let mut mux = Mux::new();
    mux.push_stream(listener.as_raw_fd());

    let server_fd = listener.as_raw_fd();
    loop {
        match mux.poll(-1) {
            Ok(ready_fds) => {
                for fd in ready_fds {
                    if fd == server_fd {
                        match new_connection(&listener) {
                            Ok(stream) => mux.push_stream(stream),
                            Err(e) => {
                                panic!("{}", e);
                            }
                        }
                    } else {
                        let stream = unsafe { TcpStream::from_raw_fd(fd) };
                        let frame = read_frame(&stream);

                        let mut reader = BufReader::new(WsStream::new(frame, &stream));

                        match frame.opcode {
                            Opcode::Continuation => {}
                            Opcode::Text => {
                                let mut str = String::new();
                                reader
                                    .read_to_string(&mut str)
                                    .expect("failed to read from reader");
                            }
                            Opcode::Binary => {
                                let mut buf: Vec<u8> = vec![];
                                reader.read_to_end(&mut buf).expect("failed to read to end");
                            }
                            Opcode::Reserved => {}
                            Opcode::Close => {
                                let mut buf = [0u8; 2];
                                reader
                                    .read_exact(&mut buf)
                                    .expect("filed to read from reader");
                                let status_code = u16::from_be_bytes(buf);
                                let frame = Frame::new(true, Opcode::Close, None, 2);
                                let mut wstream = WsStream::new(frame, &stream);
                                wstream.write(&u16::to_be_bytes(status_code)).unwrap();
                                mux.remove_pfd(fd);
                                mem::drop(stream);
                                continue;
                            }
                            Opcode::Ping => {
                                let mut buf: Vec<u8> = vec![];
                                let n =
                                    reader.read_to_end(&mut buf).expect("failed to read to end");
                                let pong = Frame::new(true, Opcode::Pong, None, n);
                                let mut stream = WsStream::new(pong, &stream);
                                stream.write(&buf[0..n]).unwrap();
                            }
                            Opcode::Pong => {
                                continue;
                            }
                        }

                        mem::forget(stream);
                    }
                }
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Opcode {
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
struct Frame {
    is_final: bool,
    opcode: Opcode,
    mask: Option<[u8; 4]>,
    payload_length: usize,
}

impl Frame {
    fn new(is_final: bool, opcode: Opcode, mask: Option<[u8; 4]>, payload_length: usize) -> Self {
        Self {
            is_final,
            opcode,
            mask,
            payload_length,
        }
    }

    fn to_blob(&self) -> Vec<u8> {
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
}

fn read_frame(mut stream: &TcpStream) -> Frame {
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

    Frame {
        is_final,
        opcode: opcode.into(),
        mask: if mask { Some(buffer) } else { None },
        payload_length: real_len,
    }
}

fn new_connection(listener: &TcpListener) -> Result<c_int, Error> {
    let mut ctx = sha1::Sha1Ctx::new();
    match listener.accept() {
        Ok((mut stream, _)) => {
            let mut key = get_key(&mut stream);
            key.push_str(GUID);
            let key = ctx.digest(&key);
            let key = base64::encode(&key);
            let accept = format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n", key);
            println!(
                "new connection from {:?} . key = {:?}",
                stream.peer_addr(),
                key
            );
            stream.write_all(accept.as_bytes()).unwrap();
            let fd = stream.as_raw_fd();
            mem::forget(stream);
            Ok(fd)
        }
        Err(e) => Err(e),
    }
}

fn get_key(stream: &mut TcpStream) -> String {
    let mut buffer = String::new();
    let mut reader = BufReader::new(stream);
    let mut key: Option<String> = None;
    loop {
        match reader.read_line(&mut buffer) {
            Ok(size) => {
                if size == 2 {
                    break;
                }
                let fields = buffer.trim().split(": ").collect::<Vec<&str>>();
                if fields[0] == "Sec-WebSocket-Key" {
                    key = Some(fields[1].to_string());
                }
                buffer.clear();
            }
            Err(e) => {
                panic!("{}", e)
            }
        }
    }
    key.unwrap()
}

struct WsStream<'a> {
    frame: Frame,
    stream: &'a TcpStream,
    buffer: Vec<u8>,
    cursor: usize,
}

impl<'a> WsStream<'a> {
    fn new(frame: Frame, stream: &'a TcpStream) -> Self {
        Self {
            frame,
            stream,
            cursor: 0,
            buffer: frame.to_blob(),
        }
    }
}

impl<'a> Read for WsStream<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.cursor == self.frame.payload_length {
            return Ok(0);
        }
        let n = self.stream.read(buf)?;
        match self.frame.mask {
            Some(mask) => {
                for c in 0..n {
                    buf[self.cursor] = buf[c] ^ mask[self.cursor % 4];
                    self.cursor += 1;
                }
            }
            None => {}
        }
        Ok(n)
    }
}

impl<'a> Write for WsStream<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.cursor + buf.len() > self.frame.payload_length {
            return Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                "too long input",
            ));
        }
        if self.cursor == 0 {
            self.flush().unwrap()
        }
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
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let result = self.stream.write_all(&self.buffer);
        self.buffer.clear();
        result
    }
}
