use std::net::{TcpListener, TcpStream};

mod clock;
use clock::Clock;

struct Player {
    stream: TcpStream,
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
                    let player = Player { stream };
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

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {

    let mut clock = Clock::new(50);
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
