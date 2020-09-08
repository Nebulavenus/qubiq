use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: ServerCfg,
    pub simulation: SimulationCfg,
    pub world: WorldCfg,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerCfg {
    pub ip: SocketAddrV4,
    pub name: String,
    pub motd: String,
    pub max_players: i8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SimulationCfg {
    pub server_tick_rate: u64,
    pub sand_tick_rate: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WorldCfg {
    pub gen: WorldGenCfg,
    pub path: String,
    pub autosave: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum WorldGenCfg {
    FromFile(String),
    FlatMap {
        width: i16,
        height: i16,
        length: i16,
    },
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerCfg {
                ip: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 25565),
                name: "Qubiq Server!".to_string(),
                motd: "Welcome to server!".to_string(),
                max_players: 10,
            },
            simulation: SimulationCfg {
                server_tick_rate: 50,
                sand_tick_rate: 20,
            },
            world: WorldCfg {
                gen: WorldGenCfg::FlatMap {
                    width: 64,
                    height: 32,
                    length: 64,
                },
                path: "maps/test.cw".to_string(),
                autosave: true,
            },
        }
    }
}

impl Config {
    pub fn new() -> anyhow::Result<Self> {
        let mut config = Config::default();
        match std::fs::read_to_string("config.yaml") {
            Ok(read_str) => {
                config = serde_yaml::from_str(&read_str)?;
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    let wr_str = serde_yaml::to_string(&config)?;
                    std::fs::write("config.yaml", wr_str)?;
                } else {
                    panic!(e);
                }
            }
        }

        Ok(config)
    }
}
