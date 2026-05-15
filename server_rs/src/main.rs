extern crate core;

mod engine_bindings;
mod protocol;

use crate::engine_bindings::{Direction, Engine, Ship, TurnResult};
use crate::protocol::{
    MSG_ERROR, MSG_JOIN, MSG_MOVE, MSG_OK, MSG_OPP_MOVE, MSG_PLAYER, MSG_READY, MSG_SHIPS,
    MSG_TURN_RESULT, recv_msg, send_msg,
};
use std::env;
use std::ffi::c_int;
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::FromRawFd;

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

// sockets variable
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
    // create a socket, return fs, fail return -1
    fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    // set socket options
    fn setsockopt(
        sockfd: c_int,
        level: c_int,
        optname: c_int,
        optval: *const std::ffi::c_void,
        optlen: u32,
    ) -> c_int;
    // bind socket to ip and port
    fn bind(sockfd: c_int, addr: *const std::ffi::c_void, addrlen: u32) -> c_int;
    // start listen requests
    fn listen(sockfd: c_int, backlog: c_int) -> c_int;
    // close socket
    fn close(fd: c_int) -> c_int;
}

fn create_listener(port: u16) -> std::io::Result<TcpListener> {
    unsafe {
        let fd: i32 = socket(AF_INET, SOCK_STREAM, 0);
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }

        // set SO_REUSEADDR options
        let enable: c_int = 1; // port reuse
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

        // bind localhost port
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

fn accept_player(
    listener: &TcpListener,
    game_id: u32,
    engine: &mut Engine,
    is_first: bool, // use it to decide init game or not
) -> std::io::Result<TcpStream> {
    // wait util all join
    let (mut stream, _addr) = listener.accept()?;

    // read client join messages
    let (msg_type, payload) = recv_msg(&mut stream)?;

    if msg_type != MSG_JOIN || payload.len() != 4 {
        let _ = send_msg(&mut stream, MSG_ERROR, &[]);
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "expected JOIN with 4-byte game_id",
        ));
    }

    let join_id = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    if join_id != game_id {
        let _ = send_msg(&mut stream, MSG_ERROR, &[]);
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "wrong game_id",
        ));
    }

    // first player touch game init
    if is_first {
        engine.init_game(game_id);
    }

    // reply OK
    send_msg(&mut stream, MSG_OK, &[])?;
    Ok(stream)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <port>", args[0]);
    }

    let port: u16 = args[1].parse().expect("invalid port");
    dprintln!("Use Port: {}", port);

    // init engine
    let mut engine: Engine = match Engine::new() {
        Some(e) => e,
        None => {
            eprintln!("Failed to initialise engine");
            std::process::exit(2);
        }
    };
    dprintln!("Engine Successfully Initialized");

    // create tcp listener
    let listener = match create_listener(port) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Socket error: {}", e);
            std::process::exit(3);
        }
    };
    dprintln!("TCP Listener Successfully Created");

    // TASK1: game id fix 0
    let game_id: u32 = 0;

    // accept p1
    let mut p1 = match accept_player(&listener, game_id, &mut engine, true) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to accept player 1: {}", e);
            std::process::exit(4);
        }
    };
    dprintln!("P1 Accepted");

    // accept p2
    let mut p2 = match accept_player(&listener, game_id, &mut engine, false) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to accept player 2: {}", e);
            std::process::exit(4);
        }
    };
    dprintln!("P2 Accepted");

    // notify both game ready
    let _ = send_msg(&mut p1, MSG_READY, &[]);
    let _ = send_msg(&mut p2, MSG_READY, &[]);

    // TASK2: ship placement
    // Receive ships from both players, place via engine (p1 first to get number 1).
    let p1_ships = match recv_ships(&mut p1) {
        Ok(s) => s,
        Err(_) => {
            let _ = send_msg(&mut p1, MSG_ERROR, &[]);
            let _ = send_msg(&mut p2, MSG_ERROR, &[]);
            engine.end_game(game_id);
            return;
        }
    };
    let p2_ships = match recv_ships(&mut p2) {
        Ok(s) => s,
        Err(_) => {
            let _ = send_msg(&mut p1, MSG_ERROR, &[]);
            let _ = send_msg(&mut p2, MSG_ERROR, &[]);
            engine.end_game(game_id);
            return;
        }
    };

    let n1 = engine.place_ships(game_id, &p1_ships);
    let n2 = engine.place_ships(game_id, &p2_ships);
    if n1 < 0 || n2 < 0 {
        let _ = send_msg(&mut p1, MSG_ERROR, &[]);
        let _ = send_msg(&mut p2, MSG_ERROR, &[]);
        engine.end_game(game_id);
        return;
    }
    let _ = send_msg(&mut p1, MSG_PLAYER, &[n1 as u8]);
    let _ = send_msg(&mut p2, MSG_PLAYER, &[n2 as u8]);

    // TASK3: gameplay loop, p1 moves first
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
        if msg_type != MSG_MOVE || payload.is_empty() {
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
        let result = engine.take_turn(game_id, player_num, &coord);
        let result_byte = (result as i8) as u8;

        let _ = send_msg(&mut streams[active], MSG_TURN_RESULT, &[result_byte]);

        let mut opp_body: Vec<u8> = Vec::with_capacity(2 + payload.len());
        opp_body.push(payload.len() as u8);
        opp_body.extend_from_slice(&payload);
        opp_body.push(result_byte);
        let _ = send_msg(&mut streams[other], MSG_OPP_MOVE, &opp_body);

        if matches!(result, TurnResult::Win) || matches!(result, TurnResult::Invalid) {
            break;
        }
        active = other;
    }

    engine.end_game(game_id);

    // keep server running
    loop {
        std::thread::park();
    }
}

// Parse a SHIPS message into 4 ships.
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
