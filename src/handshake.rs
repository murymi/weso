use std::{io::{BufRead, BufReader, Error, Write}, net::{TcpListener, TcpStream}};

use crate::{base64, frame::GUID, sha1};

pub fn new_connection(listener: &TcpListener) -> Result<TcpStream, Error> {
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
            Ok(stream)
        }
        Err(e) => Err(e),
    }
}

pub fn get_key(stream: &mut TcpStream) -> String {
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
