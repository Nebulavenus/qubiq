use crate::packets::{self, ClientPacket, ServerPacket};
use crate::packets::{
    CLIENT_BLOCK, CS_IDENTIFICATION, CS_MESSAGE, CS_PING_PONG, CS_POSITION_ORIENTATION,
};
use std::collections::VecDeque;
use std::net::TcpStream;

pub struct Player {
    pub stream: TcpStream,
    pub active: bool,

    pub pid: i8,
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
            yaw: 0,
            pitch: 0,
            operator: 0,
            authed: false,
        })
    }

    // TODO(nv): find better way?
    pub fn tick(
        &mut self,
        spawn_queue: &mut VecDeque<crate::packets::PID>,
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

                            // Spawn authed player in the middle of the world
                            let mut world_center = world.spawning_center();
                            world_center.1 += 51;
                            packets::spawn_player(
                                self.stream.try_clone()?,
                                ServerPacket::SpawnPlayer {
                                    pid: -1, // always self
                                    username: self.name.clone(),
                                    position: world_center,
                                    yaw: self.yaw,
                                    pitch: self.pitch,
                                },
                            )?;

                            // Send to spawn queue
                            spawn_queue.push_back(self.pid);
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
                            println!("Pos: {:?} - Yaw: {} - Pitch: {}", position, yaw, pitch);
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

    pub fn spawn_player(
        &self,
        player: &Player,
        world: Option<&mut crate::World>,
    ) -> anyhow::Result<()> {
        // Spawn player for a self.player, if world passed then in the middle of the world
        let mut data = ServerPacket::SpawnPlayer {
            pid: player.pid,
            username: player.name.clone(),
            position: player.position,
            yaw: player.yaw,
            pitch: player.pitch,
        };
        if let Some(world) = world {
            let mut world_center = world.spawning_center();
            world_center.1 += 51;
            if let ServerPacket::SpawnPlayer {
                ref mut position, ..
            } = data
            {
                *position = world_center;
            }
        }
        packets::spawn_player(self.stream.try_clone()?, data)?;
        Ok(())
    }

    pub fn broadcast_position(&self, player: &Player) -> anyhow::Result<()> {
        packets::player_position_update(
            self.stream.try_clone()?,
            ServerPacket::PositionAndOrientation {
                pid: player.pid,
                position: player.position,
                yaw: player.yaw,
                pitch: player.pitch,
            },
        )?;

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
