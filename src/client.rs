use std::{
    io::{self, BufRead, BufReader, Write},
    net::TcpStream,
};

use frame::GUID;
use sha1::Sha1Ctx;
mod base64;
mod frame;
mod sha1;
mod stream;

pub fn connect(addr: &str) -> io::Result<TcpStream> {
    let mut stream = TcpStream::connect(addr)?;
    let dummy_key = "dGhlIHNhbXBsZSBub25jZQ==";
    //let dummy_mask = [0u8, 1, 2, 3];
    let req = format!(
        "GET /chat HTTP/1.1\r\nSec-WebSocket-Key: {}\r\n\r\n",
        dummy_key
    );
    stream.write_all(req.as_bytes())?;

    let mut ctx = Sha1Ctx::new();

    let mut result = String::new();
    let mut reader = BufReader::new(stream);
    let mut accepted = false;
    loop {
        let n = reader.read_line(&mut result).expect("failed to read line");
        let splits = result.split(": ").collect::<Vec<&str>>();
        match splits.get(0) {
            Some(&"Sec-WebSocket-Accept") => match splits.get(1) {
                Some(value) => {
                    if base64::decode(value.trim()) == ctx.digest(&format!("{dummy_key}{}", GUID)) {
                        accepted = true;
                    }
                }
                None => {}
            },
            _ => {}
        }
        if n == 2 {
            break;
        }
        result.clear();
    }

    match accepted {
        true => Ok(reader.into_inner()),
        false => Err(io::ErrorKind::ConnectionAborted.into()),
    }
}

fn main() {
    // println!("=============client==========");
// 
    // let stream = connect("127.0.0.1:3000").unwrap();
// 
    // let _ = WsStream::new(stream)
    //     .fin()
    //     .text()
    //     .mask(Some([0, 0, 0, 0]))
    //     .with_len(11)
    //     .write_all("hello world".as_bytes());
// 
    // println!("=== wrote ===");
// 
    // sleep(Duration::from_secs(5));
}
