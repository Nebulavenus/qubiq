use std::io::{Read, Write};

pub const PROTOCOL_VERSION: u8 = 0x7;

const SERVER_LEVEL_INIT: u8 = 0x02;
const SERVER_LEVEL_DATA: u8 = 0x03;
const SERVER_LEVEL_FINAL: u8 = 0x04;
const SERVER_BLOCK: u8 = 0x06;
const SERVER_SPAWN: u8 = 0x07;
const SERVER_DESPAWN: u8 = 0x0c;
const SERVER_KICK: u8 = 0x0e;
const SERVER_USER_TYPE: u8 = 0x0f;

pub const CS_IDENTIFICATION: u8 = 0x00;
pub const CS_PING_PONG: u8 = 0x01;
pub const CS_POSITION_ORIENTATION: u8 = 0x08;
pub const CS_MESSAGE: u8 = 0x0d;

pub const CLIENT_BLOCK: u8 = 0x05;

pub type PID = i8;

pub enum ClientPacket {
    PlayerAuth {
        protocol_version: u8,
        username: String,
        verification_key: String,
        unused: u8,
    },
    Message(String),
    PositionAndOrientation {
        pid: u8,
        position: (i16, i16, i16),
        yaw: u8,
        pitch: u8,
    },
    SetBlock {
        coords: (i16, i16, i16),
        mode: u8,
        block_type: u8,
    },
}

pub fn handle_player_identification<R: Read>(mut reader: &mut R) -> anyhow::Result<ClientPacket> {
    //let mut reader = BufReader::new(stream);

    // Read identification
    let protocol_version = read_byte(&mut reader)?;
    let username = read_string(&mut reader)?;
    let verification_key = read_string(&mut reader)?;
    let unused = read_byte(&mut reader)?;
    println!("Protocol version: {}", protocol_version);
    println!("Username: {}", username);
    println!("Key: {}", verification_key);
    println!("Unused: {}", unused);

    Ok(ClientPacket::PlayerAuth {
        protocol_version,
        username,
        verification_key,
        unused,
    })
}

pub fn handle_player_message<R: Read>(mut reader: &mut R) -> anyhow::Result<ClientPacket> {
    //let mut reader = BufReader::new(stream);

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

    Ok(ClientPacket::Message(back_message))
}

pub fn handle_set_block<R: Read>(mut reader: &mut R) -> anyhow::Result<ClientPacket> {
    //let mut reader = BufReader::new(stream);

    let x = read_short(&mut reader)?;
    let y = read_short(&mut reader)?;
    let z = read_short(&mut reader)?;
    let mode = read_byte(&mut reader)?;
    let block_type = read_byte(&mut reader)?;

    Ok(ClientPacket::SetBlock {
        coords: (x, y, z),
        mode,
        block_type,
    })
}

pub fn handle_position_and_orientation<R: Read>(
    mut reader: &mut R,
) -> anyhow::Result<ClientPacket> {
    //let mut reader = BufReader::new(stream);

    let pid = read_byte(&mut reader)?; // should always be 255
    let x = read_short(&mut reader)?;
    let y = read_short(&mut reader)?;
    let z = read_short(&mut reader)?;
    let yaw = read_byte(&mut reader)?;
    let pitch = read_byte(&mut reader)?;

    Ok(ClientPacket::PositionAndOrientation {
        pid,
        position: (x, y, z),
        yaw,
        pitch,
    })
}

pub enum ServerPacket<'a> {
    ServerInfo {
        operator: u8,
    },
    SpawnPlayer {
        pid: i8,
        username: String,
        position: (i16, i16, i16),
        yaw: u8,
        pitch: u8,
    },
    LevelInit,
    LevelData {
        length: i16,
        data: &'a [u8],
        percentage: u8,
    },
    LevelFinal {
        width: i16,
        height: i16,
        length: i16,
    },
    PositionAndOrientation {
        pid: i8,
        position: (i16, i16, i16),
        yaw: u8,
        pitch: u8,
    },
}

pub fn server_info<W: Write>(mut writer: &mut W, data: ServerPacket) -> anyhow::Result<()> {
    //let mut writer = BufWriter::new(stream);

    if let ServerPacket::ServerInfo { operator } = data {
        // Send back information
        write_byte(&mut writer, CS_IDENTIFICATION)?;
        write_byte(&mut writer, PROTOCOL_VERSION)?;
        write_string(&mut writer, format!("My Cool Server"))?;
        write_string(&mut writer, format!("Welcome To Server!"))?;
        write_byte(&mut writer, operator)?; // is player op(0x64) or not(0x0)
        writer.flush()?;
    }

    Ok(())
}

pub fn player_position_update<W: Write>(
    mut writer: &mut W,
    data: ServerPacket,
) -> anyhow::Result<()> {
    //let mut writer = BufWriter::new(stream);

    if let ServerPacket::PositionAndOrientation {
        pid,
        position,
        yaw,
        pitch,
    } = data
    {
        write_byte(&mut writer, CS_POSITION_ORIENTATION)?;
        write_sbyte(&mut writer, pid)?;
        write_short(&mut writer, position.0)?;
        write_short(&mut writer, position.1)?;
        write_short(&mut writer, position.2)?;
        write_byte(&mut writer, yaw)?;
        write_byte(&mut writer, pitch)?;
        writer.flush()?;
    }

    Ok(())
}

pub fn spawn_player<W: Write>(mut writer: &mut W, data: ServerPacket) -> anyhow::Result<()> {
    //let mut writer = BufWriter::new(stream);

    if let ServerPacket::SpawnPlayer {
        pid,
        username,
        position,
        yaw,
        pitch,
    } = data
    {
        write_byte(&mut writer, SERVER_SPAWN)?;
        write_sbyte(&mut writer, pid)?;
        write_string(&mut writer, username)?;
        write_short(&mut writer, position.0)?;
        write_short(&mut writer, position.1)?;
        write_short(&mut writer, position.2)?;
        write_byte(&mut writer, yaw)?;
        write_byte(&mut writer, pitch)?;
        writer.flush()?;
    } else {
        // TODO(nv): return error?
    }

    Ok(())
}

pub fn level_init<W: Write>(mut writer: &mut W, data: ServerPacket) -> anyhow::Result<()> {
    //let mut writer = BufWriter::new(stream);

    if let ServerPacket::LevelInit = data {
        // Level initialize
        write_byte(&mut writer, SERVER_LEVEL_INIT)?;
        writer.flush()?;
    } else {
        // error
    }

    Ok(())
}

pub fn level_chunk_data<W: Write>(mut writer: &mut W, data: ServerPacket) -> anyhow::Result<()> {
    //let mut writer = BufWriter::new(stream);

    if let ServerPacket::LevelData {
        length,
        data,
        percentage,
    } = data
    {
        // Basic stuff
        write_byte(&mut writer, SERVER_LEVEL_DATA)?;
        write_short(&mut writer, length)?; // chunk length

        // Chunk must be fixed size of 1024 bytes, fill the rest
        writer.write(data)?;
        for _i in 0..1024 - length {
            write_byte(&mut writer, 0x00)?;
        }

        write_byte(&mut writer, percentage)?;
        writer.flush()?;
    }

    Ok(())
}

pub fn level_finalize<W: Write>(mut writer: &mut W, data: ServerPacket) -> anyhow::Result<()> {
    //let mut writer = BufWriter::new(stream);

    if let ServerPacket::LevelFinal {
        width,
        height,
        length,
    } = data
    {
        // Level finalize
        write_byte(&mut writer, SERVER_LEVEL_FINAL)?;
        write_short(&mut writer, width)?;
        write_short(&mut writer, height)?;
        write_short(&mut writer, length)?;
        writer.flush()?;
    }

    Ok(())
}

pub fn ping<W: Write>(mut writer: &mut W) -> anyhow::Result<()> {
    //let mut writer = BufWriter::new(stream);

    write_byte(&mut writer, CS_PING_PONG)?;
    writer.flush()?;
    Ok(())
}

pub fn kick<W: Write>(mut writer: &mut W, message: String) -> anyhow::Result<()> {
    //let mut writer = BufWriter::new(stream);

    write_byte(&mut writer, SERVER_KICK)?;
    write_string(&mut writer, message)?;
    writer.flush()?;
    Ok(())
}

pub fn broadcast_message<W: Write>(mut writer: &mut W, message: String) -> anyhow::Result<()> {
    //let mut writer = BufWriter::new(stream);

    write_byte(&mut writer, CS_MESSAGE)?;
    write_sbyte(&mut writer, 0)?;
    write_string(&mut writer, message)?;
    writer.flush()?;
    Ok(())
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

pub fn write_short<W: Write>(writer: &mut W, val: i16) -> anyhow::Result<()> {
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
