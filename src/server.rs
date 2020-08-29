use std::net::TcpListener;

use crate::Player;

pub struct Server {
    // server specific
    pub running: bool,
    listener: TcpListener,

    // game specific
    pub players: Vec<Player>,
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
            player.tick(idx as u32)?;
        }

        Ok(())
    }
}