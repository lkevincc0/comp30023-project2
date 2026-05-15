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
// Client → Server: Submit Ships
const MSG_SHIPS: u8 = 0x02;
// Client → Server: Make Move
const MSG_MOVE: u8 = 0x03;

// Server → Client: OK
const MSG_OK: u8 = 0x81;
// Server → Client: ERROR
#[allow(dead_code)]
const MSG_ERROR: u8 = 0x82;
// Server → Client: READY
const MSG_READY: u8 = 0x83;
// Server → Client: assigned player number
const MSG_PLAYER: u8 = 0x84;
// Server → Client: own move result
const MSG_TURN_RESULT: u8 = 0x85;
// Server → Client: opponent's move and result
const MSG_OPP_MOVE: u8 = 0x86;

// Decode a 1-byte TurnResult value.
fn decode_turn_result(b: u8) -> TurnResult {
    match b as i8 {
        0 => TurnResult::Miss,
        1 => TurnResult::Hit,
        2 => TurnResult::Win,
        4 => TurnResult::Sunk2,
        5 => TurnResult::Sunk3,
        6 => TurnResult::Sunk4,
        7 => TurnResult::Sunk5,
        _ => TurnResult::Invalid,
    }
}

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
    let payload_len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

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
            }
            Err(e) => {
                dprintln!("TCP connection failed: {}", e);
                return false;
            }
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

    // TASK2: send ships, receive assigned player number
    pub fn send_ships(&mut self, ships: &[Ship; 4]) -> i8 {
        let stream = match self.stream.as_mut() {
            Some(s) => s,
            None => return -1,
        };

        // Serialize ships
        let mut payload: Vec<u8> = Vec::new();
        for ship in ships {
            let bytes = ship.coordinate.as_bytes();
            if bytes.len() > u8::MAX as usize {
                return -1;
            }
            payload.push(bytes.len() as u8);
            payload.extend_from_slice(bytes);
            payload.push(ship.length);
            payload.push(ship.direction as u8);
        }

        if send_msg(stream, MSG_SHIPS, &payload).is_err() {
            return -1;
        }

        let (msg_type, body) = match recv_msg(stream) {
            Ok(m) => m,
            Err(_) => return -1,
        };
        if msg_type != MSG_PLAYER || body.len() != 1 {
            return -1;
        }
        body[0] as i8
    }

    // TASK3: send move, receive turn result
    pub fn send_move(&mut self, coordinate: &str) -> TurnResult {
        let stream = match self.stream.as_mut() {
            Some(s) => s,
            None => return TurnResult::Invalid,
        };
        if send_msg(stream, MSG_MOVE, coordinate.as_bytes()).is_err() {
            return TurnResult::Invalid;
        }
        let (msg_type, body) = match recv_msg(stream) {
            Ok(m) => m,
            Err(_) => return TurnResult::Invalid,
        };
        if msg_type != MSG_TURN_RESULT || body.len() != 1 {
            return TurnResult::Invalid;
        }
        decode_turn_result(body[0])
    }

    pub fn send_move_extended(&mut self, _coordinate: &str) -> Option<Box<[u8]>> {
        None
    }

    // TASK3: block until opponent move arrives
    pub fn receive_move(&mut self) -> MoveResult {
        let invalid = MoveResult {
            coordinate: None,
            result: TurnResult::Invalid,
        };
        let stream = match self.stream.as_mut() {
            Some(s) => s,
            None => return invalid,
        };
        let (msg_type, body) = match recv_msg(stream) {
            Ok(m) => m,
            Err(_) => return invalid,
        };
        if msg_type != MSG_OPP_MOVE || body.is_empty() {
            return invalid;
        }
        let clen = body[0] as usize;
        if 1 + clen + 1 > body.len() {
            return invalid;
        }
        let coord = match std::str::from_utf8(&body[1..1 + clen]) {
            Ok(s) => s.to_string(),
            Err(_) => return invalid,
        };
        let result = decode_turn_result(body[1 + clen]);
        MoveResult {
            coordinate: Some(coord),
            result,
        }
    }

    pub fn close(&mut self) {
        // stream close when drop
        self.stream = None;
    }
}
