use std::net::TcpListener;
use std::collections::VecDeque;

use crate::Player;
use crate::packets::broadcast_message;

pub struct Server {
    // server specific
    pub running: bool,
    listener: TcpListener,

    // game specific
    pub players: Vec<Player>,
    chat: VecDeque<String>,
}

impl Server {
    pub fn new() -> anyhow::Result<Self> {
        let address = format!("127.0.0.1:80");
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;

        Ok(Server {
            running: true,
            listener,
            players: vec![],
            chat: VecDeque::new(),
        })
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        // Accept new connections
        for inc in self.listener.incoming() {
            let _ = match inc {
                Ok(stream) => {
                    let player = Player::new(stream)?;
                    self.players.push(player);
                },
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        break; // just to out of blocking-forloop to process ticking server
                    } else {
                        panic!(e);
                    }
                },
            };
        }

        // Progress world

        // Progress players
        for (idx, player) in self.players.iter_mut().enumerate() {
            // Tick player
            player.tick(idx as u32, &mut self.chat)?;
        }

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