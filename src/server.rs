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

        // Tested for now 1024x32x1024
        // TODO(nv): world type generation from config or load it from file
        let world = World::new(64, 32, 64);

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

        // Delete inactive players -- see line 68
        self.players.retain(|p| p.active == true);

        // TODO(nv): Progress world & physics
        //self.world.tick()?;

        // Progress players
        for player in self.players.iter_mut() {
            // Try to send ping and determine if the socket is still open
            // Basicaly the idea is simple. Every write or read on socket may fail. But with every tick
            // TcpStream will be checked for closed socket and mark it inactive to delete player later and not to process him.
            player.check_liveness();

            // Tick player if he is alive
            if player.active {
                match player.tick(&mut self.queue, &mut self.world) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Player: {} - Err: {}", player.pid, e);
                        // Mark it inactive and continue
                        player.active = false; // Almost never happens? At least here
                        continue;
                    }
                }
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
                            player.spawn_player(inc_player, Some(&mut self.world));

                            // Spawn for a new player other already existing players
                            inc_player.spawn_player(player, None);
                        }
                    }
                }
                packets::Queue::DespawnPlayer(_) => {
                    todo!("Despawn player");
                }
                packets::Queue::ChatMessage(msg) => {
                    for player in self.players.iter_mut() {
                        // Yeah I know... but ¯\_(ツ)_/¯
                        match packets::broadcast_message(&mut player.stream, msg.clone()) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                }
                packets::Queue::SetBlock { coords, block_type } => {
                    for player in self.players.iter_mut() {
                        match packets::broadcast_block(
                            &mut player.stream,
                            packets::ServerPacket::SetBlock { coords, block_type },
                        ) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                }
            }
        }

        // Broadcast player positions
        for o_player in self.players.iter() {
            for r_player in self.players.iter() {
                if o_player.pid == r_player.pid {
                    continue;
                }
                o_player.broadcast_position(r_player);
            }
        }

        Ok(())
    }
}
