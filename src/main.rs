mod nbt;
mod packets;
mod util;
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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    // Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Setup server specific stuff
    let config = Config::new()?; // TODO(nv): specify arg to config file for loading?
    let mut clock = Clock::new(config.simulation.server_tick_rate as u128);
    let mut server = Server::new(config.clone())?;

    println!("Started server!");

    loop {
        // Exit if not running
        if !running.load(Ordering::SeqCst) {
            if config.world.autosave {
                server.world.save_world(config.world.path.clone())?;
            }
            server.kick_players();
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

    println!("Server closed!");

    Ok(())
}
