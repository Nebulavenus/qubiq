mod packets;
mod world;
use world::World;

mod clock;
use clock::Clock;

mod player;
use player::Player;

mod server;
use server::Server;

mod config;
use config::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::new()?; // TODO(nv): specify arg to config file for loading?
    let mut clock = Clock::new(config.simulation.server_tick_rate as u128);
    let mut server = Server::new(config)?;

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
