use crate::packets::{self, ClientPacket, ServerPacket};
use crate::packets::{
    CLIENT_BLOCK, CS_IDENTIFICATION, CS_MESSAGE, CS_PING_PONG, CS_POSITION_ORIENTATION,
};
use std::collections::VecDeque;
use std::net::TcpStream;

pub struct Player {
    pub stream: TcpStream,
    pub active: bool,

    name: String,
    position: (i16, i16, i16),
    yaw: u8,
    pitch: u8,
    operator: u8,
}

impl Player {
    pub fn new(stream: TcpStream) -> anyhow::Result<Self> {
        Ok(Player {
            stream,
            active: true,
            name: String::from("Unknown"),
            position: (0, 0, 0),
            yaw: 45u8,
            pitch: 45u8,
            operator: 0,
        })
    }

    // TODO(nv): add reference to server shared state? instead passing argument
    pub fn tick(
        &mut self,
        idx: u32,
        chat: &mut VecDeque<String>,
        world: &mut crate::World,
    ) -> anyhow::Result<()> {
        println!("Player tick: {}", idx);

        let mut packet_id = None;
        match packets::read_byte(&mut self.stream) {
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
                    let data = packets::handle_player_identification(self.stream.try_clone()?)?;
                    match data {
                        ClientPacket::PlayerAuth {
                            protocol_version,
                            username,
                            verification_key,
                            unused,
                        } => {
                            // TODO: handle invalid protocol version
                            if protocol_version != crate::packets::PROTOCOL_VERSION {
                                println!(
                                    "Protocol version mismatch! Client is {} - Server is {}",
                                    protocol_version,
                                    crate::packets::PROTOCOL_VERSION
                                );

                                // TODO(nv): kick user
                            }

                            // Set player nickname
                            self.name.clone_from(&username.trim_end().to_string()); // also trim whitespaces
                            self.name.shrink_to_fit();

                            // TODO(nv): authenticate with md5
                            // TODO(nv): kick if server is full

                            // TODO(nv): set operator type
                            self.operator = 0x64;

                            // Send server info after successful auth
                            packets::server_info(
                                self.stream.try_clone()?,
                                ServerPacket::ServerInfo {
                                    operator: self.operator,
                                },
                            )?;

                            // Send world information
                            world.send_world(self.stream.try_clone()?)?;

                            let world_center = world.spawning_center();
                            /*
                            spawn_player(
                                self.stream.try_clone()?,
                                -1,
                                self.name.clone(),
                                world_center.0,
                                world_center.1,
                                world_center.2,
                                45,
                                45,
                            )?;
                            */
                        }
                        _ => unreachable!(),
                    }
                }
                CS_POSITION_ORIENTATION => {
                    let data = packets::handle_position_and_orientation(self.stream.try_clone()?)?;
                    match data {
                        ClientPacket::PositionAndOrientation {
                            pid,
                            position,
                            yaw,
                            pitch,
                        } => {
                            self.position = position;
                            self.yaw = yaw;
                            self.pitch = pitch;
                        }
                        _ => unreachable!(),
                    }
                }
                CS_MESSAGE => {
                    let data = packets::handle_player_message(self.stream.try_clone()?)?;
                    match data {
                        ClientPacket::Message(msg) => {
                            // Save it in server's chat to broadcast it later
                            let mut formatted = format!("{}: ", self.name.clone());
                            formatted.push_str(&msg);
                            println!("{}", formatted);
                            chat.push_back(formatted); // could overflow - not 64 length
                        }
                        _ => unreachable!(),
                    }
                }
                CS_PING_PONG => println!("Player pong"), // never returns - just to check if i can write to socket
                CLIENT_BLOCK => {
                    let data = packets::handle_set_block(self.stream.try_clone()?)?;
                    match data {
                        ClientPacket::SetBlock {
                            coords,
                            mode,
                            block_type,
                        } => {}
                        _ => unreachable!(),
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    pub fn check_liveness(&mut self) -> anyhow::Result<()> {
        match packets::ping(self.stream.try_clone()?) {
            Ok(_) => {}
            Err(e) => match e.downcast::<std::io::Error>() {
                Ok(err) => {
                    if err.kind() == std::io::ErrorKind::ConnectionAborted {
                        self.active = false;
                    }
                }
                Err(pe) => panic!(pe),
            },
        }
        Ok(())
    }

    pub fn disconnect(&mut self, reason: String) -> anyhow::Result<()> {
        packets::kick(self.stream.try_clone()?, reason)?;
        Ok(())
    }
}
