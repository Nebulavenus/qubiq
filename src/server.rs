use std::collections::VecDeque;
use std::net::TcpListener;

use crate::packets::broadcast_message;
use crate::Player;
use crate::World;

pub struct Server {
    // server specific
    pub running: bool,
    listener: TcpListener,

    // game specific
    pub players: Vec<Player>,
    chat: VecDeque<String>,
    world: World,
}

impl Server {
    pub fn new() -> anyhow::Result<Self> {
        let address = format!("127.0.0.1:80");
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;

        // TODO(nv): world type generation from config or load it
        let world = World::new(10, 10, 10);

        Ok(Server {
            running: true,
            listener,
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
                    let player = Player::new(stream)?;
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
        for (idx, player) in self.players.iter_mut().enumerate() {
            // Send ping to determine if socket is open
            player.check_liveness()?;

            // Tick player if he is alive
            if player.active {
                player.tick(idx as u32, &mut self.chat, &mut self.world)?;
            }
        }

        // Delete unactive
        self.players.retain(|p| p.active == true);

        // Broadcast messages
        while let Some(msg) = self.chat.pop_back() {
            for player in self.players.iter_mut() {
                broadcast_message(player.stream.try_clone()?, msg.clone())?;
                // TODO(nv): could panic on write operation if player's stream closed
            }
        }

        Ok(())
    }
}
