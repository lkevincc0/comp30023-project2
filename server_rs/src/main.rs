// MULTIPLE_GAMES
extern crate core;

mod engine_bindings;
mod protocol;

use crate::engine_bindings::{Direction, Engine, Ship, TurnResult};
use crate::protocol::{
    MSG_ERROR, MSG_JOIN, MSG_MOVE, MSG_MOVE_EXT, MSG_OK, MSG_OPP_MOVE, MSG_PLAYER, MSG_READY,
    MSG_SHIPS, MSG_TURN_RESULT, MSG_TURN_RESULT_EXT, recv_msg, send_msg,
};
use std::collections::HashMap;
use std::env;
use std::ffi::c_int;
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::FromRawFd;
use std::sync::{Arc, Mutex};
use std::thread;

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

#[cfg(target_os = "linux")]
const SOL_SOCKET: c_int = 1;
#[cfg(target_os = "linux")]
const SO_REUSEADDR: c_int = 2;
#[cfg(target_os = "macos")]
const SOL_SOCKET: c_int = 0xffff;
#[cfg(target_os = "macos")]
const SO_REUSEADDR: c_int = 0x0004;
const AF_INET: c_int = 2;
const SOCK_STREAM: c_int = 1;

unsafe extern "C" {
    fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    fn setsockopt(
        sockfd: c_int,
        level: c_int,
        optname: c_int,
        optval: *const std::ffi::c_void,
        optlen: u32,
    ) -> c_int;
    fn bind(sockfd: c_int, addr: *const std::ffi::c_void, addrlen: u32) -> c_int;
    fn listen(sockfd: c_int, backlog: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

fn create_listener(port: u16) -> std::io::Result<TcpListener> {
    unsafe {
        let fd: i32 = socket(AF_INET, SOCK_STREAM, 0);
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let enable: c_int = 1;
        if setsockopt(
            fd,
            SOL_SOCKET,
            SO_REUSEADDR,
            &enable as *const c_int as *const std::ffi::c_void,
            std::mem::size_of::<c_int>() as u32,
        ) < 0
        {
            close(fd);
            return Err(std::io::Error::last_os_error());
        }
        let mut addr = [0u8; 16];
        addr[0..2].copy_from_slice(&(AF_INET as u16).to_ne_bytes());
        addr[2..4].copy_from_slice(&port.to_be_bytes());
        if bind(fd, addr.as_ptr() as *const std::ffi::c_void, 16) < 0 {
            close(fd);
            return Err(std::io::Error::last_os_error());
        }
        if listen(fd, 128) < 0 {
            close(fd);
            return Err(std::io::Error::last_os_error());
        }
        Ok(TcpListener::from_raw_fd(fd))
    }
}

type EngineRef = Arc<Mutex<Engine>>;
type Pending = Arc<Mutex<HashMap<u32, Vec<TcpStream>>>>;

fn handle_connection(mut stream: TcpStream, pending: Pending, engine: EngineRef) {
    let (msg_type, payload) = match recv_msg(&mut stream) {
        Ok(m) => m,
        Err(_) => {
            let _ = send_msg(&mut stream, MSG_ERROR, &[]);
            return;
        }
    };
    if msg_type != MSG_JOIN || payload.len() != 4 {
        let _ = send_msg(&mut stream, MSG_ERROR, &[]);
        return;
    }
    let game_id = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);

    // Register this player and possibly take both streams to run the game.
    let pair: Option<(TcpStream, TcpStream)> = {
        let mut map = pending.lock().unwrap();
        let slot = map.entry(game_id).or_default();
        if slot.len() >= 2 {
            let _ = send_msg(&mut stream, MSG_ERROR, &[]);
            return;
        }
        let is_first = slot.is_empty();
        if is_first && !engine.lock().unwrap().init_game(game_id) {
            // Another game with this id is still active; reject without leaking the slot.
            map.remove(&game_id);
            let _ = send_msg(&mut stream, MSG_ERROR, &[]);
            return;
        }
        if send_msg(&mut stream, MSG_OK, &[]).is_err() {
            if is_first {
                engine.lock().unwrap().end_game(game_id);
                map.remove(&game_id);
            }
            return;
        }
        slot.push(stream);
        if slot.len() == 2 {
            let mut taken = std::mem::take(slot);
            map.remove(&game_id);
            let p2 = taken.pop().unwrap();
            let p1 = taken.pop().unwrap();
            Some((p1, p2))
        } else {
            None
        }
    };

    if let Some((p1, p2)) = pair {
        run_game(game_id, p1, p2, engine);
    }
}

fn run_game(game_id: u32, mut p1: TcpStream, mut p2: TcpStream, engine: EngineRef) {
    let _ = send_msg(&mut p1, MSG_READY, &[]);
    let _ = send_msg(&mut p2, MSG_READY, &[]);

    // Ship placement
    let p1_ships = match recv_ships(&mut p1) {
        Ok(s) => s,
        Err(_) => {
            let _ = send_msg(&mut p1, MSG_ERROR, &[]);
            let _ = send_msg(&mut p2, MSG_ERROR, &[]);
            engine.lock().unwrap().end_game(game_id);
            return;
        }
    };
    let p2_ships = match recv_ships(&mut p2) {
        Ok(s) => s,
        Err(_) => {
            let _ = send_msg(&mut p1, MSG_ERROR, &[]);
            let _ = send_msg(&mut p2, MSG_ERROR, &[]);
            engine.lock().unwrap().end_game(game_id);
            return;
        }
    };

    let (n1, n2) = {
        let mut e = engine.lock().unwrap();
        (
            e.place_ships(game_id, &p1_ships),
            e.place_ships(game_id, &p2_ships),
        )
    };
    if n1 < 0 || n2 < 0 {
        let _ = send_msg(&mut p1, MSG_ERROR, &[]);
        let _ = send_msg(&mut p2, MSG_ERROR, &[]);
        engine.lock().unwrap().end_game(game_id);
        return;
    }
    let _ = send_msg(&mut p1, MSG_PLAYER, &[n1 as u8]);
    let _ = send_msg(&mut p2, MSG_PLAYER, &[n2 as u8]);

    // Gameplay loop
    let mut streams = [p1, p2];
    let mut active: usize = 0;
    loop {
        let other = 1 - active;
        let (msg_type, payload) = match recv_msg(&mut streams[active]) {
            Ok(m) => m,
            Err(_) => {
                let _ = send_msg(&mut streams[other], MSG_ERROR, &[]);
                break;
            }
        };
        if (msg_type != MSG_MOVE && msg_type != MSG_MOVE_EXT) || payload.is_empty() {
            let _ = send_msg(&mut streams[active], MSG_ERROR, &[]);
            let _ = send_msg(&mut streams[other], MSG_ERROR, &[]);
            break;
        }
        let coord = match std::str::from_utf8(&payload) {
            Ok(s) => s.to_string(),
            Err(_) => {
                let _ = send_msg(&mut streams[active], MSG_ERROR, &[]);
                let _ = send_msg(&mut streams[other], MSG_ERROR, &[]);
                break;
            }
        };

        let player_num = (active as u8) + 1;
        let result: TurnResult = if msg_type == MSG_MOVE_EXT {
            let (data, r) = {
                let mut e = engine.lock().unwrap();
                let d = e.take_turn_extended(game_id, player_num, &coord);
                let r = e.extract_turn_result(&d);
                (d, r)
            };
            let mut ext_body: Vec<u8> = Vec::with_capacity(2 + data.len());
            ext_body.extend_from_slice(&(data.len() as u16).to_be_bytes());
            ext_body.extend_from_slice(&data);
            let _ = send_msg(&mut streams[active], MSG_TURN_RESULT_EXT, &ext_body);
            r
        } else {
            let r = engine
                .lock()
                .unwrap()
                .take_turn(game_id, player_num, &coord);
            let _ = send_msg(&mut streams[active], MSG_TURN_RESULT, &[(r as i8) as u8]);
            r
        };

        let mut opp_body: Vec<u8> = Vec::with_capacity(2 + payload.len());
        opp_body.push(payload.len() as u8);
        opp_body.extend_from_slice(&payload);
        opp_body.push((result as i8) as u8);
        let _ = send_msg(&mut streams[other], MSG_OPP_MOVE, &opp_body);

        if matches!(result, TurnResult::Win) || matches!(result, TurnResult::Invalid) {
            break;
        }
        active = other;
    }

    engine.lock().unwrap().end_game(game_id);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <port>", args[0]);
        std::process::exit(1);
    }
    let port: u16 = match args[1].parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Invalid port: {}", args[1]);
            std::process::exit(5);
        }
    };
    dprintln!("Use Port: {}", port);

    let engine = match Engine::new() {
        Some(e) => Arc::new(Mutex::new(e)),
        None => {
            eprintln!("Failed to initialise engine");
            std::process::exit(2);
        }
    };
    dprintln!("Engine Successfully Initialized");

    let listener = match create_listener(port) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Socket error: {}", e);
            // Drop the Arc so the Engine's Drop runs engine_free.
            drop(engine);
            std::process::exit(3);
        }
    };
    dprintln!("TCP Listener Successfully Created");

    let pending: Pending = Arc::new(Mutex::new(HashMap::new()));

    // Accept loop: spawn a thread per connection so a hanging client doesn't block others.
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let pending = pending.clone();
                let engine = engine.clone();
                thread::spawn(move || handle_connection(stream, pending, engine));
            }
            Err(e) => {
                dprintln!("accept failed: {}", e);
                continue;
            }
        }
    }
}

fn recv_ships(stream: &mut std::net::TcpStream) -> std::io::Result<[Ship; 4]> {
    let (msg_type, payload) = recv_msg(stream)?;
    if msg_type != MSG_SHIPS {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "expected SHIPS",
        ));
    }
    let mut ships: Vec<Ship> = Vec::with_capacity(4);
    let mut i = 0;
    for _ in 0..4 {
        if i >= payload.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "short ships payload",
            ));
        }
        let clen = payload[i] as usize;
        i += 1;
        if i + clen + 2 > payload.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "short ships payload",
            ));
        }
        let coord = std::str::from_utf8(&payload[i..i + clen])
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad coord utf8"))?
            .to_string();
        i += clen;
        let length = payload[i];
        i += 1;
        let direction = match payload[i] {
            0 => Direction::Horizontal,
            1 => Direction::Vertical,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "bad direction",
                ));
            }
        };
        i += 1;
        ships.push(Ship {
            coordinate: coord,
            length,
            direction,
        });
    }
    Ok([
        ships.remove(0),
        ships.remove(0),
        ships.remove(0),
        ships.remove(0),
    ])
}
