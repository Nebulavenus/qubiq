use crate::packets::{self, ClientPacket, ServerPacket};
use crate::packets::{
    CLIENT_BLOCK, CS_IDENTIFICATION, CS_MESSAGE, CS_PING_PONG, CS_POSITION_ORIENTATION,
};
use std::collections::VecDeque;
use std::net::TcpStream;

pub struct Player {
    pub stream: TcpStream,
    pub active: bool,

    pid: i8,
    name: String,
    position: (i16, i16, i16),
    yaw: u8,
    pitch: u8,
    operator: u8,
    authed: bool,
}

impl Player {
    pub fn new(stream: TcpStream, pid: i8) -> anyhow::Result<Self> {
        Ok(Player {
            stream,
            active: true,
            pid,
            name: String::from("Unknown"),
            position: (0, 0, 0),
            yaw: 45u8,
            pitch: 45u8,
            operator: 0,
            authed: false,
        })
    }

    // TODO(nv): add reference to server shared state? instead passing argument
    pub fn tick(
        &mut self,
        players: &mut Vec<Player>,
        chat: &mut VecDeque<String>,
        world: &mut crate::World,
    ) -> anyhow::Result<()> {
        // First try to read first opcode
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
                            if protocol_version != crate::packets::PROTOCOL_VERSION {
                                let msg = format!(
                                    "Protocol version mismatch! Your: {} - Server: {}",
                                    protocol_version,
                                    packets::PROTOCOL_VERSION
                                );
                                self.disconnect(msg)?;
                                return Ok(());
                            }

                            // Set player nickname
                            self.name.clone_from(&username.trim_end().to_string()); // also trim whitespaces
                            self.name.shrink_to_fit();

                            // TODO(nv): authenticate with md5
                            // TODO(nv): kick if server is full

                            // TODO(nv): set operator type
                            self.operator = 0x64;

                            // Authed
                            self.authed = true;

                            // Send server info after successful auth
                            packets::server_info(
                                self.stream.try_clone()?,
                                ServerPacket::ServerInfo {
                                    operator: self.operator,
                                },
                            )?;

                            // Send world information
                            world.send_world(self.stream.try_clone()?)?;

                            // TODO(nv): move this out?
                            // TODO(nv): send after new player connected, for now probably redundant
                            // Send spawn position
                            let world_center = world.spawning_center();
                            packets::spawn_player(
                                self.stream.try_clone()?,
                                ServerPacket::SpawnPlayer {
                                    pid: -1, // always self
                                    username: self.name.clone(),
                                    position: world_center,
                                    yaw: 45,
                                    pitch: 45,
                                },
                            )?;
                            // Another player
                            packets::spawn_player(
                                self.stream.try_clone()?,
                                ServerPacket::SpawnPlayer {
                                    pid: 1, // always self
                                    username: self.name.clone(),
                                    position: world_center,
                                    yaw: 45,
                                    pitch: 45,
                                },
                            )?;
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

                            // First packet to myself
                            packets::player_position_update(
                                self.stream.try_clone()?,
                                ServerPacket::PositionAndOrientation {
                                    pid: -1, // myself
                                    position: self.position,
                                    yaw: self.yaw,
                                    pitch: self.pitch,
                                },
                            )?;

                            // Rebroadcast to other players
                            for player in players {
                                if player.pid == self.pid {
                                    continue;
                                }

                                packets::player_position_update(
                                    player.stream.try_clone()?,
                                    ServerPacket::PositionAndOrientation {
                                        pid: player.pid,
                                        position: player.position,
                                        yaw: player.yaw,
                                        pitch: player.pitch,
                                    },
                                )?;
                            }
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
