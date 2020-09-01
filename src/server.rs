use std::collections::VecDeque;
use std::net::TcpListener;

use crate::packets;
use crate::Player;
use crate::World;

pub struct Server {
    // server specific
    pub running: bool,
    listener: TcpListener,

    // events(values) that must be processed later after every player ticked
    pub queue: VecDeque<packets::Queue>,

    // game specific
    pub players: Vec<Player>,
    pub world: World,
}

impl Server {
    pub fn new() -> anyhow::Result<Self> {
        let address = format!("127.0.0.1:80");
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;

        // TODO(nv): world type generation from config or load it from file
        let world = World::new(10, 10, 10);

        Ok(Server {
            running: true,
            listener,
            queue: VecDeque::new(),
            players: vec![],
            world,
        })
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        // Accept new connections
        for inc in self.listener.incoming() {
            let _ = match inc {
                Ok(stream) => {
                    // TODO(nv): handle pid more correctly
                    let player_count = self.players.len();
                    let player = Player::new(stream, player_count as i8)?;
                    self.players.push(player);
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        break; // just to out of blocking-forloop to process ticking server
                    } else {
                        panic!(e);
                    }
                }
            };
        }

        // TODO(nv): Progress world & physics
        //self.world.tick()?;

        // Progress players
        for player in self.players.iter_mut() {
            // Send ping to determine if socket is open
            player.check_liveness()?;

            // Tick player if he is alive
            if player.active {
                player.tick(&mut self.queue, &mut self.world)?;
            }
        }

        println!("Queue to process: {}", self.queue.len());
        // Process events queue
        while let Some(ev_queue) = self.queue.pop_back() {
            match ev_queue {
                // Player spawner
                // If a new player connects send it to others and also send current players to him
                packets::Queue::SpawnPlayer(pid) => {
                    if let Some(inc_player) = self.players.iter().find(|c| c.pid == pid) {
                        for player in self.players.iter() {
                            if player.pid == pid {
                                continue;
                            }

                            // Spawn in the middle
                            player.spawn_player(inc_player, Some(&mut self.world))?;

                            // Spawn for a new player other already existing players
                            inc_player.spawn_player(player, None)?;
                        }
                    }
                }
                packets::Queue::DespawnPlayer(_) => {
                    todo!("Despawn player");
                }
                packets::Queue::ChatMessage(msg) => {
                    for player in self.players.iter_mut() {
                        // TODO(nv): could panic on write operation if player's stream closed
                        packets::broadcast_message(&mut player.stream, msg.clone())?;
                    }
                }
                packets::Queue::SetBlock { coords, block_type } => {
                    for player in self.players.iter_mut() {
                        packets::broadcast_block(
                            &mut player.stream,
                            packets::ServerPacket::SetBlock { coords, block_type },
                        )?;
                    }
                }
            }
        }

        // TODO(nv): check connection in packet class on each write and disconnect it?
        // Delete inactive players
        self.players.retain(|p| p.active == true);

        // Broadcast player positions
        for o_player in self.players.iter() {
            for r_player in self.players.iter() {
                if o_player.pid == r_player.pid {
                    continue;
                }
                o_player.broadcast_position(r_player)?;
            }
        }

        Ok(())
    }
}
