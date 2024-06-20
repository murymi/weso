use std::{io::{BufRead, BufReader, Read}, net::{TcpListener, TcpStream}};

const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();

    for mut stream in listener.incoming() {
        let key = get_key(stream.as_mut().unwrap());
        println!("new connection from {:?} . key = {:?}", stream.as_ref().unwrap().peer_addr(), key);
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
            },
            Err(e) => {
                panic!("{}", e)
            },
        }
    }
    key.unwrap()
}

fn write_accept(stream: &mut TcpStream, mut key:String) {
    let concatenation = key.push_str(GUID);
    
}


//                  1001