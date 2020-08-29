use std::net::{TcpListener, TcpStream};

mod packets;

mod clock;
use clock::Clock;

struct Player {
    offline: bool,
    stream: TcpStream,
}

impl Player {
    fn tick(&mut self, idx: u32) -> anyhow::Result<()> {
        if self.offline { return Ok(()); }

        println!("Player tick: {}", idx);
        
        let mut packet_id: u8 = 0xFF;
        match packets::read_byte(&mut self.stream) {
            Ok(v) => packet_id = v,
            Err(e) => {
                match e.downcast::<std::io::Error>() {
                    Ok(er) => {
                        if er.kind() == std::io::ErrorKind::WouldBlock {
                            // Just return function and don't handle incoming packets to avoid panic
                            return Ok(());
                        }
                    }
                    Err(e) => { panic!(e); }
                }
            }
        }

        //let packet_id = packets::read_byte(&mut self.stream)?;
        println!("Received packet_id: {}", packet_id);
        println!("");
        match packet_id {
            0x0 => packets::handle_player_identification(self.stream.try_clone()?)?,
            //0xd => handle_player_message(self.stream.clone())?,
            _ => (),
        }

        //self.offline = true;

        Ok(())
    }
}

struct Server {
    // server specific
    running: bool,
    listener: TcpListener,

    // game specific
    players: Vec<Player>,
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
                    let player = Player { offline: false, stream };
                    self.players.push(player);
                },
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        break;
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

fn main() -> anyhow::Result<()> {

    let mut clock = Clock::new(500);
    let mut server = Server::new()?;

    println!("Started server!");

    loop {
        // Exit if not running
        if !server.running { break; }

        // Start clocker
        clock.start();

        // Progress server ticks
        server.tick()?;
        println!("Players count: {}", server.players.len());

        // Count ticks
        clock.finish_tick();
    }

    Ok(())
}
