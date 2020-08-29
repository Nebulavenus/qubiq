use std::net::TcpStream;
use crate::packets::*;

pub struct Player {
    offline: bool,
    stream: TcpStream,
}

impl Player {
    pub fn new(stream: TcpStream) -> anyhow::Result<Self> {
        Ok(Player {
            offline: false,
            stream,
        })
    }

    pub fn tick(&mut self, idx: u32) -> anyhow::Result<()> {
        if self.offline { return Ok(()); }

        println!("Player tick: {}", idx);
        
        let mut packet_id: u8 = 0xFF;
        match read_byte(&mut self.stream) {
            Ok(v) => packet_id = v,
            Err(e) => {
                match e.downcast::<std::io::Error>() {
                    Ok(er) => {
                        if er.kind() == std::io::ErrorKind::WouldBlock {
                            // Just return function and don't handle incoming packets to avoid panic
                            return Ok(());
                        }
                    }
                    Err(e) => { panic!(e); }
                }
            }
        }

        //let packet_id = packets::read_byte(&mut self.stream)?;
        println!("Received packet_id: {}", packet_id);
        println!("");
        match packet_id {
            0x0 => handle_player_identification(self.stream.try_clone()?)?,
            0xd => handle_player_message(self.stream.try_clone()?)?,
            _ => (),
        }

        //self.offline = true;

        Ok(())
    }
}