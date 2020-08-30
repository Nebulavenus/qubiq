use crate::packets::*;
use std::collections::VecDeque;
use std::net::TcpStream;

pub struct Player {
    name: String,
    pub active: bool,
    pub stream: TcpStream,
}

impl Player {
    pub fn new(stream: TcpStream) -> anyhow::Result<Self> {
        Ok(Player {
            name: String::from("Unknown"),
            active: true,
            stream,
        })
    }

    // TODO(nv): add reference to server shared state? instead passing argument
    pub fn tick(&mut self, idx: u32, chat: &mut VecDeque<String>) -> anyhow::Result<()> {
        println!("Player tick: {}", idx);

        let mut packet_id = None;
        match read_byte(&mut self.stream) {
            Ok(v) => packet_id = Some(v),
            Err(e) => {
                match e.downcast::<std::io::Error>() {
                    Ok(er) => {
                        if er.kind() == std::io::ErrorKind::WouldBlock {
                            // Just return function and don't handle incoming packets to avoid panic
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        panic!(e);
                    }
                }
            }
        }

        if let Some(packet_id) = packet_id {
            // TODO(nv): handle send back parsed enums to handle state here instead.
            println!("Received packet_id: {}", packet_id);
            println!("");
            match packet_id {
                CS_IDENTIFICATION => {
                    handle_player_identification(self.stream.try_clone()?, &mut self.name)?
                }
                CS_MESSAGE => {
                    handle_player_message(self.stream.try_clone()?, self.name.clone(), chat)?
                }
                //CS_POSITION_ORIENTATION =>
                CS_PING_PONG => println!("Player pong"), // never returns - just to check if i can write to socket
                _ => (),
            }
        }

        Ok(())
    }

    pub fn check_liveness(&mut self) -> anyhow::Result<()> {
        match ping(self.stream.try_clone()?) {
            Ok(_) => {}
            Err(e) => {
                match e.downcast::<std::io::Error>() {
                    Ok(err) => {
                        if err.kind() == std::io::ErrorKind::ConnectionAborted {
                            self.active = false;
                        }
                        //println!("ERRR{:?}", err.kind());
                    }
                    Err(pe) => panic!(pe),
                }
            }
        }
        Ok(())
    }

    pub fn disconnect(&mut self, reason: String) -> anyhow::Result<()> {
        kick(self.stream.try_clone()?, reason)?;
        Ok(())
    }
}
