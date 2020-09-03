use std::collections::VecDeque;
use std::net::TcpListener;

use crate::packets;
use crate::Player;
use crate::World;

pub enum Queue {
    SpawnPlayer(i8),
    DespawnPlayer(i8),
    ChatMessage(String),
    SetBlock {
        coords: (i16, i16, i16),
        block_type: u8,
    },
}

pub struct Server {
    // server specific
    pub running: bool,
    listener: TcpListener,
    max_players: i8,

    // events(values) that must be processed later after every player ticked
    pub queue: VecDeque<Queue>,

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
            max_players: 1, // TODO(nv): move it out to config
            queue: VecDeque::new(),
            players: vec![],
            world,
        })
    }

    fn gen_pid(&self) -> Option<i8> {
        for id in 0..=i8::MAX {
            let mut free = true;
            for player in self.players.iter() {
                if player.pid == id {
                    free = false;
                    break;
                }
            }
            if free {
                return Some(id);
            }
        }
        None
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        // Accept new connections
        for inc in self.listener.incoming() {
            let _ = match inc {
                Ok(stream) => {
                    let mut player = Player::new(stream, -1);

                    // Players count
                    let current_players = self.players.len() as i8;
                    if current_players + 1 > self.max_players {
                        match player.disconnect("Server is full!".to_string()) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }

                    // Gen pid then add incoming player to players list
                    if let Some(pid) = self.gen_pid() {
                        player.pid = pid;
                        self.players.push(player);
                    } else {
                        // Server is full kick! max pid
                        match player.disconnect("Server is full!".to_string()) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
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
            } else {
                // If not active then despawn for other players
                self.queue.push_back(Queue::DespawnPlayer(player.pid));
            }
        }

        println!("Queue to process: {}", self.queue.len());
        // Process events queue
        while let Some(ev_queue) = self.queue.pop_back() {
            match ev_queue {
                // Player spawner
                // If a new player connects send it to others and also send current players to him
                Queue::SpawnPlayer(pid) => {
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
                Queue::DespawnPlayer(pid) => {
                    // Despawn inactive player for others
                    for player in self.players.iter_mut() {
                        match packets::despawn_player(
                            &mut player.stream,
                            packets::ServerPacket::DespawnPlayer(pid),
                        ) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                }
                Queue::ChatMessage(msg) => {
                    for player in self.players.iter_mut() {
                        // Yeah I know... but ¯\_(ツ)_/¯
                        match packets::broadcast_message(
                            &mut player.stream,
                            packets::ServerPacket::Message(msg.clone()),
                        ) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                }
                Queue::SetBlock { coords, block_type } => {
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
