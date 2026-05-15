use std::io::{Read, Result, Write};

// client to server
pub const MSG_JOIN: u8 = 0x01;
pub const MSG_SHIPS: u8 = 0x02;
pub const MSG_MOVE: u8 = 0x03;

// server to client
/// OK
pub const MSG_OK: u8 = 0x81;
/// ERROR
pub const MSG_ERROR: u8 = 0x82;
/// READY
pub const MSG_READY: u8 = 0x83;
/// Player number assignment
pub const MSG_PLAYER: u8 = 0x84;
/// Turn result for active player
pub const MSG_TURN_RESULT: u8 = 0x85;
/// Opponent move + result for waiting player
pub const MSG_OPP_MOVE: u8 = 0x86;

pub fn send_msg(stream: &mut dyn Write, msg_type: u8, payload: &[u8]) -> Result<()> {
    let mut buf = Vec::with_capacity(5 + payload.len());
    buf.push(msg_type);
    buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    buf.extend_from_slice(payload);
    stream.write_all(&buf)?;
    stream.flush()
}

pub fn recv_msg(stream: &mut dyn Read) -> Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 5];
    stream.read_exact(&mut header)?;

    // read header
    let msg_type = header[0];

    let payload_len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;

    Ok((msg_type, payload))
}
