use std::collections::VecDeque;
use std::net::TcpListener;

use crate::packets::broadcast_message;
use crate::Player;
use crate::World;

pub struct Server {
    // server specific
    pub running: bool,
    listener: TcpListener,

    // kinda callbacks
    pub spawn_queue: VecDeque<crate::packets::PID>,
    pub despawn_queue: VecDeque<crate::packets::PID>,

    // game specific
    pub players: Vec<Player>,
    pub chat: VecDeque<String>,
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
            spawn_queue: VecDeque::new(),
            despawn_queue: VecDeque::new(),
            players: vec![],
            chat: VecDeque::new(),
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
                player.tick(&mut self.spawn_queue, &mut self.chat, &mut self.world)?;
            }
        }

        // Player spawner
        // If a new player connects send it to others and also send current players to him
        while let Some(pid) = self.spawn_queue.pop_back() {
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

        println!("CHAT MSG: {}", self.chat.len());

        // Broadcast messages
        while let Some(msg) = self.chat.pop_back() {
            for player in self.players.iter_mut() {
                broadcast_message(&mut player.stream, msg.clone())?;

                // TODO(nv): could panic on write operation if player's stream closed
            }
        }

        Ok(())
    }
}
