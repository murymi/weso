use std::{
    io::{BufReader, Read},
    net::TcpListener,
};

use frame::Opcode;
use handshake::new_connection;
use mux::Mux;

mod base64;
mod frame;
mod mux;
mod sha1;
mod stream;
mod handshake;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();
    let mut mux = Mux::with_listener(listener);

    //let server_fd = listener.as_raw_fd();
    loop {
        match mux.poll(-1) {
            Ok(event) => {
                match event {
                    mux::Event::Join(listener) => match new_connection(&listener) {
                        Ok(stream) => {
                            mux.push_stream(stream);
                        }
                        Err(e) => {
                            panic!("{}", e);
                        }
                    },
                    mux::Event::Ready(ready_streams) => {
                        for mut stream in ready_streams.into_iter() {
                            //stream.read_frame();
                            //let mut reader = BufReader::new(&mut stream);
                            match stream.opcode() {
                                Opcode::Continuation => {
                                    println!("continue from: {:?}", stream.peer_addr());
                                    //sleep(Duration::from_secs(60));
                                }
                                Opcode::Text => {
                                    println!("text from: {:?}", stream.peer_addr());
                                    let mut str = String::new();
                                    let n = stream.read_to_string(&mut str).expect("failed to read");
                                    println!("message:[{n}] {}", str);
                                    if str == "kwenda senji" {
                                        stream.text("nkwende nkwile ku", None).expect("failed to echo");
                                    }
                                }
                                Opcode::Binary => {
                                    println!("binary from: {:?}", stream.peer_addr());
                                    let mut buf: Vec<u8> = vec![];
                                    let mut reader = BufReader::new(&mut stream);
                                    reader.read_to_end(&mut buf).expect("failed to read to end");
                                }
                                Opcode::Reserved => {}
                                Opcode::Close => {
                                    println!("close from: {:?}", stream.peer_addr());
                                    stream.bye(None).unwrap();
                                    mux.remove(stream);
                                    continue;
                                }
                                Opcode::Ping => {
                                    println!("ping from: {:?}", stream.peer_addr());
                                    stream.pong(None);
                                }
                                Opcode::Pong => {
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }
}

