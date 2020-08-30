use std::collections::VecDeque;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;

const PROTOCOL_VERSION: u8 = 0x7;

pub const SERVER_LEVEL_INIT: u8 = 0x02;
pub const SERVER_LEVEL_DATA: u8 = 0x03;
pub const SERVER_LEVEL_FINAL: u8 = 0x04;
pub const SERVER_BLOCK: u8 = 0x06;
pub const SERVER_SPAWN: u8 = 0x07;
pub const SERVER_DESPAWN: u8 = 0x0c;
pub const SERVER_KICK: u8 = 0x0e;
pub const SERVER_USER_TYPE: u8 = 0x0f;

pub const CS_IDENTIFICATION: u8 = 0x00;
pub const CS_PING_PONG: u8 = 0x01;
pub const CS_POSITION_ORIENTATION: u8 = 0x08;
pub const CS_MESSAGE: u8 = 0x0d;

pub const CLIENT_BLOCK: u8 = 0x05;

pub fn handle_player_identification(
    stream: TcpStream,
    player_name: &mut String,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream.try_clone()?);

    // Read identification
    let protocol_version = read_byte(&mut reader)?;
    let username = read_string(&mut reader)?;
    let verification_key = read_string(&mut reader)?;
    let unused = read_byte(&mut reader)?;
    println!("Protocol version: {}", protocol_version);
    println!("Username: {}", username);
    println!("Key: {}", verification_key);
    println!("Unused: {}", unused);

    // TODO: handle invalid protocol version
    if protocol_version != PROTOCOL_VERSION {
        println!(
            "Protocol version mismatch! Client is {} - Server is {}",
            protocol_version, PROTOCOL_VERSION
        );
    }

    // Set player nickname
    player_name.clone_from(&username.trim_end().to_string());
    player_name.shrink_to_fit();

    // Send back information
    write_byte(&mut writer, CS_IDENTIFICATION)?;
    write_byte(&mut writer, protocol_version)?;
    write_string(&mut writer, format!("My Cool Server"))?;
    write_string(&mut writer, format!("Welcome To Server!"))?;
    write_byte(&mut writer, 0x64)?; // is player op(0x64) or not(0x0)
    writer.flush()?;

    // Send ping
    write_byte(&mut writer, CS_PING_PONG)?;
    writer.flush()?;

    // Level initialize
    write_byte(&mut writer, SERVER_LEVEL_INIT)?;
    writer.flush()?;

    // Level finalize
    write_byte(&mut writer, SERVER_LEVEL_FINAL)?;
    write_short(&mut writer, 0)?;
    write_short(&mut writer, 0)?;
    write_short(&mut writer, 0)?;
    writer.flush()?;

    Ok(())
}

pub fn handle_player_message(
    stream: TcpStream,
    player_nick: String,
    chat: &mut VecDeque<String>,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    //let mut writer = BufWriter::new(stream.try_clone()?);

    // Get message from client
    let _unused = read_byte(&mut reader)?;
    let message = read_string(&mut reader)?;

    println!("Client message: {}", &message);

    // Replace % to be colored
    let mut back_message = message.replace("%", "&");
    // Sanitize string, if it contains & at end it crashes.
    if back_message.ends_with("&") {
        back_message.pop();
    }

    // Save it to broadcast it later
    let mut formatted = format!("{}: ", player_nick);
    formatted.push_str(&back_message);
    println!("{}", formatted);
    chat.push_back(formatted); // could overflow - not 64 length

    Ok(())
}

pub fn ping(stream: TcpStream) -> anyhow::Result<()> {
    let mut writer = BufWriter::new(stream.try_clone()?);

    // Send ping
    write_byte(&mut writer, CS_PING_PONG)?;
    writer.flush()?;
    Ok(())
}

pub fn kick(stream: TcpStream, message: String) -> anyhow::Result<()> {
    let mut writer = BufWriter::new(stream.try_clone()?);

    write_byte(&mut writer, SERVER_KICK)?;
    write_string(&mut writer, message)?;
    writer.flush()?;
    Ok(())
}

pub fn broadcast_message(stream: TcpStream, message: String) -> anyhow::Result<()> {
    //let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream.try_clone()?);

    write_byte(&mut writer, CS_MESSAGE)?;
    write_sbyte(&mut writer, 0)?;
    write_string(&mut writer, message)?;
    writer.flush()?;
    Ok(())
}

pub struct Packet {
    packet_id: u8,
    packet_len: usize,
    data: Vec<u8>,
}

pub fn read_byte<R: Read>(reader: &mut R) -> anyhow::Result<u8> {
    let mut buf = [0u8];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_sbyte<R: Read>(reader: &mut R) -> anyhow::Result<i8> {
    let mut buf = [0u8];
    reader.read_exact(&mut buf)?;
    Ok(buf[0] as i8)
}

fn read_short<R: Read>(reader: &mut R) -> anyhow::Result<i16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_be_bytes(buf))
}

fn read_string<R: Read>(reader: &mut R) -> anyhow::Result<String> {
    let mut buf = [0u8; 64];
    reader.read_exact(&mut buf)?;
    let res = String::from_utf8_lossy(&buf);
    Ok(res.into())
}

fn write_byte<W: Write>(writer: &mut W, val: u8) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

fn write_sbyte<W: Write>(writer: &mut W, val: i8) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

fn write_short<W: Write>(writer: &mut W, val: i16) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

fn write_string<W: Write>(writer: &mut W, val: String) -> anyhow::Result<()> {
    let mut buf = [0u8; 64];
    let val_bytes = val.as_bytes();
    let vb_len = val_bytes.len();
    if vb_len > buf.len() {
        buf.clone_from_slice(&val_bytes[..64]);
    } else {
        buf[..vb_len].clone_from_slice(&val_bytes);
    }
    writer.write(&buf)?;
    Ok(())
}
