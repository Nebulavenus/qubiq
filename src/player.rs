use std::net::TcpStream;
use crate::packets::*;
use std::collections::VecDeque;

pub struct Player {
    name: String,
    offline: bool,
    pub stream: TcpStream,
}

impl Player {
    pub fn new(stream: TcpStream) -> anyhow::Result<Self> {
        Ok(Player {
            name: String::from("Unknown"),
            offline: false,
            stream,
        })
    }

    // TODO(nv): add reference to server shared state? instead passing argument
    pub fn tick(&mut self, idx: u32, chat: &mut VecDeque<String>) -> anyhow::Result<()> {
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

        // TODO(nv): handle send back parsed enums to handle state here instead.
        println!("Received packet_id: {}", packet_id);
        println!("");
        match packet_id {
            0x0 => handle_player_identification(self.stream.try_clone()?, &mut self.name)?,
            0xd => handle_player_message(self.stream.try_clone()?, self.name.clone(), chat)?,
            _ => (),
        }

        //self.offline = true;

        Ok(())
    }
}