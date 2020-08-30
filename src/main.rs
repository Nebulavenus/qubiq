mod packets;
mod world;
use world::World;

mod clock;
use clock::Clock;

mod player;
use player::Player;

mod server;
use server::Server;

fn main() -> anyhow::Result<()> {
    let mut clock = Clock::new(500);
    let mut server = Server::new()?;

    println!("Started server!");

    loop {
        // Exit if not running
        if !server.running {
            break;
        }

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
