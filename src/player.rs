use crate::config;
use crate::packets::{self, ClientPacket, ServerPacket};
use crate::packets::{
    CLIENT_BLOCK, CS_IDENTIFICATION, CS_MESSAGE, CS_PING_PONG, CS_POSITION_ORIENTATION,
};
use crate::server;
use std::collections::VecDeque;
use std::io::{BufReader, BufWriter};
use std::net::TcpStream;

pub struct Player {
    pub stream: TcpStream,
    pub active: bool,

    pub pid: i8,
    pub name: String,
    position: (i16, i16, i16),
    yaw: u8,
    pitch: u8,
    operator: u8,
    authed: bool,
}

impl Player {
    pub fn new(stream: TcpStream, pid: i8) -> Self {
        Player {
            stream,
            active: true,
            pid,
            name: String::from("Unknown"),
            position: (0, 0, 0),
            yaw: 0,
            pitch: 0,
            operator: 0,
            authed: false,
        }
    }

    pub fn tick(
        &mut self,
        config: config::Config,
        queue: &mut VecDeque<server::Queue>,
        world: &mut crate::World,
    ) -> anyhow::Result<()> {
        // Data loss if not buffered
        let mut reader = BufReader::new(self.stream.try_clone()?);
        let mut writer = BufWriter::new(self.stream.try_clone()?);

        // TODO(nv): limit this loop if the sent data is too big -- in future
        loop {
            // First try to read first opcode
            let mut packet_id = None;
            //match packets::read_byte(&mut self.stream) {
            match packets::read_byte(&mut reader) {
                Ok(v) => packet_id = Some(v),
                Err(e) => {
                    match e.downcast::<std::io::Error>() {
                        Ok(er) => {
                            if er.kind() == std::io::ErrorKind::WouldBlock {
                                // Just return function and don't handle incoming packets to avoid panic
                                //break;
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
                // TODO(nv): just for debug purpose, 0x08 is sent very often
                if packet_id != 0x08 {
                    println!("Received packet_id: {}", packet_id);
                    println!("");
                }
                match packet_id {
                    CS_IDENTIFICATION => {
                        let data = packets::handle_player_identification(&mut reader)?;
                        match data {
                            #[allow(unused_variables)]
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

                                // TODO(nv): set operator type
                                self.operator = 0x64;

                                // Authed
                                self.authed = true;

                                // Send server info after successful auth
                                packets::server_info(
                                    &mut writer,
                                    ServerPacket::ServerInfo {
                                        operator: self.operator,
                                        name: config.server.name.clone(),
                                        motd: config.server.motd.clone(),
                                    },
                                )?;

                                // Send world information
                                world.send_world(&mut writer)?;

                                // Spawn authed player in the middle of the world
                                let mut world_center = world.spawning_center();
                                world_center.1 += 51;
                                packets::spawn_player(
                                    &mut writer,
                                    ServerPacket::SpawnPlayer {
                                        pid: -1, // always self
                                        username: self.name.clone(),
                                        position: world_center,
                                        yaw: self.yaw,
                                        pitch: self.pitch,
                                    },
                                )?;

                                // Send to spawn queue for other players
                                queue.push_back(server::Queue::SpawnPlayer(self.pid));
                                // also notify of new connection
                                queue.push_back(server::Queue::ChatMessage(format!(
                                    "&e{} joined the game",
                                    self.name.clone()
                                )));
                            }
                            _ => unreachable!(),
                        }
                    }
                    CS_POSITION_ORIENTATION => {
                        let data = packets::handle_position_and_orientation(&mut reader)?;
                        match data {
                            #[allow(unused_variables)]
                            ClientPacket::PositionAndOrientation {
                                pid,
                                position,
                                yaw,
                                pitch,
                            } => {
                                self.position = position;
                                self.yaw = yaw;
                                self.pitch = pitch;
                                //println!("Pos: {:?} - Yaw: {} - Pitch: {}", position, yaw, pitch);
                            }
                            _ => unreachable!(),
                        }
                    }
                    CS_MESSAGE => {
                        let data = packets::handle_player_message(&mut reader)?;
                        match data {
                            ClientPacket::Message(msg) => {
                                // Save it in server's chat to broadcast it later
                                let mut formatted = format!("{}: ", self.name.clone());
                                formatted.push_str(&msg);
                                println!("{}", formatted);

                                // TODO(nv): test = could overflow - not 64 length
                                // Broadcast message to other players
                                queue.push_back(server::Queue::ChatMessage(formatted));
                            }
                            _ => unreachable!(),
                        }
                    }
                    CS_PING_PONG => println!("Player pong"), // never returns - just to check if i can write to socket
                    CLIENT_BLOCK => {
                        let data = packets::handle_set_block(&mut reader)?;
                        match data {
                            ClientPacket::SetBlock {
                                coords,
                                mode,
                                block_type,
                            } => {
                                println!(
                                    "Coords: {:?} - Mode: {} - BlockType: {}",
                                    coords, mode, block_type
                                );

                                // TODO(nv): check if block is valid (loop through all known blocks u8)

                                // TODO(nv): if not valid send to air 0x0, also check world.set_block

                                // Broadcast block to other players
                                if mode == 0x0 {
                                    // block destroyed
                                    queue.push_back(server::Queue::SetBlock {
                                        coords,
                                        block_type: 0x00, // air
                                    });

                                    world.set_block(coords.0, coords.1, coords.2, 0x00);
                                } else {
                                    // else place block which player held
                                    queue.push_back(server::Queue::SetBlock { coords, block_type });

                                    world.set_block(coords.0, coords.1, coords.2, block_type);
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                    _ => unreachable!(),
                }
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn spawn_player(&self, player: &Player, world: Option<&mut crate::World>) {
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
        let mut writer = BufWriter::new(&self.stream);
        match packets::spawn_player(&mut writer, data) {
            Ok(_) => {}
            Err(_) => {}
        };
    }

    pub fn broadcast_position(&self, player: &Player) {
        let mut writer = BufWriter::new(&self.stream);
        match packets::player_position_update(
            &mut writer,
            ServerPacket::PositionAndOrientation {
                pid: player.pid,
                position: player.position,
                yaw: player.yaw,
                pitch: player.pitch,
            },
        ) {
            Ok(_) => {}
            Err(_) => {}
        };
    }

    pub fn check_liveness(&mut self) {
        match packets::ping(&mut self.stream) {
            Ok(_) => {}
            Err(e) => match e.downcast::<std::io::Error>() {
                Ok(err) => {
                    // Set inactive and delete it in next tick iteration
                    if err.kind() != std::io::ErrorKind::WouldBlock {
                        self.active = false;
                    }
                }
                Err(pe) => panic!(pe),
            },
        }
    }

    pub fn disconnect(&mut self, reason: String) -> anyhow::Result<()> {
        packets::kick(&mut self.stream, ServerPacket::Kick(reason))?;
        Ok(())
    }
}
