use std::{
    ffi::c_int,
    io::stdin,
    net::{TcpListener, TcpStream},
    os::fd::AsRawFd, thread::{sleep, spawn}, time::Duration,
};

pub enum Event {
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

impl From<Event> for i16 {
    fn from(value: Event) -> Self {
        match value {
            Event::POLLNVAL => 0x20,
            Event::POLLHUP => 0x10,
            Event::POLLERR => 0x8,
            Event::POLLOUT => 0x4,
            Event::POLLPRI => 0x2,
            Event::POLLIN => 0x1,
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

pub struct Mux {
    pfds: Vec<Pollfd>,
}

impl Mux {
    pub fn new() -> Self {
        Self { pfds: Vec::new() }
    }

    pub fn push_stream(&mut self, fd: c_int) {
        self.add_pfd(Pollfd {
            fd,
            events: Event::POLLIN.into(),
            revents: 0,
        });
    }

    fn add_pfd(&mut self, pfd: Pollfd) {
        self.pfds.push(pfd);
    }

    pub fn remove_pfd(&mut self, fd: c_int) {
        let mut index = 0;
        for pfd in &self.pfds {
            if pfd.fd == fd {
                break;
            }
            index += 1;
        }
        self.pfds.remove(index);
    }

    pub fn poll(&self, timeout: isize) -> Result<Vec<c_int>, PollErr> {
        let events = unsafe { poll(self.pfds.as_ptr(), self.pfds.len(), timeout) };
        if events == 0 {
            Err(PollErr::TimedOut)
        } else if events == EINTR {
            Err(PollErr::Interupted)
        } else if events < 0 {
            Err(PollErr::Other)
        } else {
            let mut ready = vec![];
            for pfd in &self.pfds {
                let events = pfd.revents ;
                if events & (Event::POLLIN as i16) != 0 {
                    ready.push(pfd.fd);
                }
            }
            Ok(ready)
        }
    }
}

// extern fn handler(sig: c_int) -> c_int {
//     println!("==========signal received==========");
//     //unsafe { raise(2) };
//     0
// }
// 
// fn main() {
//     //let g = vec![];
// 
//     //let f = handler
// 
//     
//     let mut mux = Mux::new();
//     mux.add_pfd(Pollfd {
//         fd: stdin().as_raw_fd(),
//         events: Event::POLLIN.into(),
//         revents: 0,
//     });
// 
//     T
//     
//     let mut index = 0;
//     
//     spawn(|| { 
//         sleep(Duration::from_secs(5));
//         unsafe { raise(10) } 
//     });
//     //.join().expect("failed to join");
//     
//     unsafe{ signal(10, handler) };
// 
//     loop {
//         //let events = unsafe { poll((&pfds).as_ptr(), 1, Duration::from_secs(3).as_millis() as u64) };
//         println!("blocking");
//         //mux.poll(-1);
//         match mux.poll(1000) {
//             Ok(r) => {
//                 println!("ready {:?}", r);
//             }
//             Err(e) =>
//             match e {
//                 PollErr::Interupted => println!("interupted"),
//                 PollErr::TimedOut => println!("timed out"),
//                 PollErr::Other => panic!("== {:?} ==", e),
//             },
//         }
//         //println!("ublcoked");
// 
//         //let duration = Duration::from_secs(3).as_secs();
// 
//         //if events < 0 {
//         //    panic!("poll failed");
//         //}
// 
//         //if events == TIMEOUT {
//         //    println!("time out ");
//         //    continue;
//         //}
// 
//         //let mut buff = [0u8; 64];
//         //stdin().read(&mut buff).unwrap();
//     }
// }
// 