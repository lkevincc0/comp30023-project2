pub mod client_bindings;

extern crate core;
use crate::client_bindings::{MoveResult, Ship, TurnResult};
use std::io::{Read, Write};
use std::net::TcpStream;

macro_rules! dprintln {
    () => {
        #[cfg(debug_assertions)]
        println!("[{}:{}]", file!(), line!());
    };
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            eprintln!(
                "[{}:{}] {}",
                file!(),
                line!(),
                format_args!($($arg)*)
            );
        }
    };
}

// Client → Server: Join Game
const MSG_JOIN: u8 = 0x01;

// Server → Client: OK
const MSG_OK: u8 = 0x81;
// Server → Client: ERROR
const MSG_ERROR: u8 = 0x82;
// Server → Client: READY
const MSG_READY: u8 = 0x83;

// send a msgs
fn send_msg(stream: &mut TcpStream, msg_type: u8, payload: &[u8]) -> std::io::Result<()> {
    let len = (payload.len() as u32).to_be_bytes();
    stream.write_all(&[msg_type])?;
    stream.write_all(&len)?;
    stream.write_all(payload)?;
    stream.flush()
}

// receive mesg
fn recv_msg(stream: &mut TcpStream) -> std::io::Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 5];
    stream.read_exact(&mut header)?;

    let msg_type = header[0];
    let payload_len =
        u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;

    Ok((msg_type, payload))
}

pub struct ClientImplementation {
    stream: Option<TcpStream>,
}

impl ClientImplementation {
    // init client
    pub fn init() -> Option<Self> {
        dprintln!("Initializing client...");
        Some(Self { stream: None })
    }

    // client connect to server and join game
    pub fn connect(&mut self, addr: &str, port: u16, game_id: u32) -> bool {
        let full_addr = format!("{}:{}", addr, port);
        dprintln!("Attempting to connect to {}", full_addr);

        // TCP Connect
        let mut stream = match TcpStream::connect(format!("{}:{}", addr, port)) {
            Ok(s) => {
                dprintln!("TCP connection established.");
                s
            },
            Err(e) => {
                dprintln!("TCP connection failed: {}", e);
                return false
            },
        };

        // send join messages
        if send_msg(&mut stream, MSG_JOIN, &game_id.to_be_bytes()).is_err() {
            return false;
        }

        // wait for server reply
        let (msg_type, _) = match recv_msg(&mut stream) {
            Ok(m) => m,
            Err(e) => {
                dprintln!("Failed to receive response from server: {:?}", e);
                return false;
            }
        };

        if msg_type != MSG_OK {
            return false;
        }

        dprintln!("Successfully joined game");
        // connected, save stream
        self.stream = Some(stream);
        true
    }

    // client wait for opponent
    pub fn wait_for_opponent(&mut self) -> bool {
        let stream = match self.stream.as_mut() {
            Some(s) => s,
            None => return false,
        };

        let (msg_type, _) = match recv_msg(stream) {
            Ok(m) => m,
            Err(_) => return false,
        };

        msg_type == MSG_READY
    }

    // TASK2 TODO

    pub fn send_ships(&mut self, _ships: &[Ship; 4]) -> i8 {
        -1
    }

    pub fn send_move(&mut self, _coordinate: &str) -> TurnResult {
        TurnResult::Invalid
    }

    pub fn send_move_extended(&mut self, _coordinate: &str) -> Option<Box<[u8]>> {
        None
    }

    pub fn receive_move(&mut self) -> MoveResult {
        MoveResult {
            coordinate: None,
            result: TurnResult::Invalid,
        }
    }

    pub fn close(&mut self) {
        // stream close when drop
        self.stream = None;
    }
}

