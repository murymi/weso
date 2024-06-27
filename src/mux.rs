use std::{
    ffi::c_int,
    net::{TcpListener, TcpStream},
    os::fd::{AsRawFd, FromRawFd, IntoRawFd},
};

use crate::stream::WsStream;

pub enum Ev {
    POLLNVAL,
    POLLHUP,
    POLLERR,
    POLLOUT,
    POLLPRI,
    POLLIN,
}

const EINTR: isize = 4;
const TIMEOUT: isize = 0;

#[derive(Debug)]
pub enum PollErr {
    Interupted,
    TimedOut,
    Other,
}

impl From<Ev> for i16 {
    fn from(value: Ev) -> Self {
        match value {
            Ev::POLLNVAL => 0x20,
            Ev::POLLHUP => 0x10,
            Ev::POLLERR => 0x8,
            Ev::POLLOUT => 0x4,
            Ev::POLLPRI => 0x2,
            Ev::POLLIN => 0x1,
        }
    }
}

#[repr(C)]
struct Pollfd {
    fd: c_int,
    events: i16,
    revents: i16,
}

extern "C" {
    fn poll(pfds: *const Pollfd, fdcount: usize, timeout: isize) -> isize;
}

pub enum Event<'a> {
    Join(&'a mut TcpListener),
    Ready(Vec<WsStream>),
}

pub struct Mux {
    pfds: Vec<Pollfd>,
    //stream_map: HashMap<c_int, WsStream>,
    listener: TcpListener,
}

impl Mux {
    //pub fn new() -> Self {
    //    Self {
    //        pfds: Vec::new(),
    //        pfd_map: HashMap::new()
    //    }
    //}

    pub fn with_listener(stream: TcpListener) -> Self {
        Self {
            pfds: vec![Pollfd {
                fd: stream.as_raw_fd(),
                events: Ev::POLLIN.into(),
                revents: 0,
            }],
            //stream_map: HashMap::new(),
            listener: stream,
        }
    }

    pub fn push_stream(&mut self, stream: TcpStream) {
        let fd = stream.into_raw_fd();
        self.add_pfd(Pollfd {
            fd,
            events: Ev::POLLIN.into(),
            revents: 0,
        });

        //self.stream_map.insert(fd, WsStream::new(stream));
    }

    fn add_pfd(&mut self, pfd: Pollfd) {
        self.pfds.push(pfd);
    }

    pub fn remove(&mut self, fd: WsStream) {
        let mut index = 0;
        let fd = fd.stream.as_raw_fd();
        for pfd in &self.pfds {
            if pfd.fd == fd {
                break;
            }
            index += 1;
        }
        self.pfds.remove(index);
        unsafe { TcpStream::from_raw_fd(fd) };
        //self.stream_map.remove(&fd.as_raw_fd());
    }

    pub fn poll(&mut self, timeout: isize) -> Result<Event, PollErr> {
        let events = unsafe { poll(self.pfds.as_ptr(), self.pfds.len(), timeout) };
        if events == 0 {
            Err(PollErr::TimedOut)
        } else if events == EINTR {
            Err(PollErr::Interupted)
        } else if events < 0 {
            Err(PollErr::Other)
        } else {
            let mut ready = vec![];
            for (i, pfd) in self.pfds.iter().enumerate() {
                let events = pfd.revents;
                if i == 0 {
                    if events & (Ev::POLLIN as i16) != 0 {
                        return Ok(Event::Join(&mut (self.listener)));
                    }
                } else {
                    if events & (Ev::POLLIN as i16) != 0 {
                        let mut stream = WsStream::new(unsafe { TcpStream::from_raw_fd(pfd.fd) });
                        stream.read_frame();
                        ready.push(stream);
                    }
                }
            }
            Ok(Event::Ready(ready))
        }
    }
}
