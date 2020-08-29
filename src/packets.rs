use std::io::{Read, Write, BufReader, BufWriter};
use std::net::TcpStream;
use std::collections::VecDeque;

pub fn handle_player_identification(stream: TcpStream, player_name: &mut String) -> anyhow::Result<()> {
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

    // Set player nickname
    player_name.clone_from(&username.trim_end().to_string());
    player_name.shrink_to_fit();

    // Send back information
    write_byte(&mut writer, 0x0)?;
    write_byte(&mut writer, protocol_version)?;
    write_string(&mut writer, format!("My Cool Server"))?;
    write_string(&mut writer, format!("Welcome To Server!"))?;
    write_byte(&mut writer, 0x64)?; // is player op(0x64) or not(0x0)
    writer.flush()?;

    // Send ping
    write_byte(&mut writer, 0x01)?;
    writer.flush()?;

    // Level initialize
    write_byte(&mut writer, 0x02)?;
    writer.flush()?;

    // Level finalize
    write_byte(&mut writer, 0x04)?;
    write_short(&mut writer, 0)?;
    write_short(&mut writer, 0)?;
    write_short(&mut writer, 0)?;
    writer.flush()?;

    Ok(())
}

pub fn handle_player_message(stream: TcpStream, player_nick: String, chat: &mut VecDeque<String>) -> anyhow::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    //let mut writer = BufWriter::new(stream.try_clone()?);

    // Get message from client
    let _unused = read_byte(&mut reader)?;
    let message = read_string(&mut reader)?;

    println!("Client message: {}", &message);

    // Replace % to be colored
    let mut back_message = message.replace("%", "&");
    // Sanitize string, if it contains & at end it crashes.
    if back_message.ends_with("&") { back_message.pop(); }

    /*
    // Send it back
    write_byte(&mut writer, 0xd)?; // serverbound packet msg
    write_sbyte(&mut writer, 0)?; // Player ID
    write_string(&mut writer, back_message.clone())?;
    writer.flush()?;
    */

    // Save it to broadcast it later
    let mut formatted = format!("{}: ", player_nick);
    formatted.push_str(&back_message);
    println!("{}", formatted);
    chat.push_back(formatted); // could overflow - not 64 length

    Ok(())
}

pub fn broadcast_message(stream: TcpStream, message: String) -> anyhow::Result<()> {
    //let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream.try_clone()?);

    write_byte(&mut writer, 0xd)?;
    write_sbyte(&mut writer, 0)?;
    write_string(&mut writer, message)?;
    writer.flush()?;
    Ok(())
}

pub struct Packet {
    packet_id: u8,
    packet_len: usize,
    data: Vec<u8>
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