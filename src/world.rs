use crate::nbt::{self, NBT};
use crate::packets::{self, ServerPacket};
use flate2::{bufread, write};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

pub struct World {
    pub width: i16,
    pub height: i16,
    pub length: i16,
    pub blocks: Vec<u8>,

    spawn: (i16, i16, i16),
}

impl World {
    pub fn new(width: i16, height: i16, length: i16) -> Self {
        let count = width as usize * height as usize * length as usize;
        let blocks = vec![0x00u8; count]; // fill with air
        let mut world = World {
            width,
            height,
            length,
            blocks,
            spawn: (width / 2, height / 2, length / 2),
        };

        // TODO(nv): make builder pattern
        world.generate_flat_map();

        world
    }

    fn generate_flat_map(&mut self) {
        // Basic algorithm
        for y in 0..self.height / 2 {
            for x in 0..self.width {
                for z in 0..self.length {
                    let idx = self.coord_to_block_idx(x, y, z);
                    if y < (self.height / 2 - 1) {
                        self.blocks[idx] = 0x03;
                    } else {
                        self.blocks[idx] = 0x02;
                    }
                }
            }
        }
    }

    fn coord_to_block_idx(&mut self, x: i16, y: i16, z: i16) -> usize {
        let x = x as usize;
        let y = y as usize;
        let z = z as usize;
        let width = self.width as usize;
        let length = self.length as usize;
        return (x + width * (z + length * y)) as usize;
    }

    pub fn set_block(&mut self, x: i16, y: i16, z: i16, block_id: u8) {
        let block = self.coord_to_block_idx(x, y, z);
        match self.blocks.get_mut(block) {
            Some(bid) => *bid = block_id,
            None => {}
        }
    }

    pub fn get_block(&mut self, x: i16, y: i16, z: i16) -> u8 {
        let block = self.coord_to_block_idx(x, y, z);
        match self.blocks.get(block) {
            Some(bid) => *bid,
            None => panic!("Cant find this block!"), // or return air
        }
    }

    pub fn load_world<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        // Load file
        let f = File::open(path)?;
        let r = BufReader::new(f);
        let mut gz = bufread::GzDecoder::new(r);
        let nbt = NBT::read(&mut gz).unwrap();

        // Parse nbt
        let mut width = 1;
        let mut height = 1;
        let mut length = 1;
        let mut blocks = Vec::new();
        let mut spawn = (1i16, 1i16, 1i16);

        if let nbt::Tag::Compound(v) = nbt.tag() {
            for (key, tag) in v {
                match key.as_str() {
                    "X" => {
                        if let nbt::Tag::Short(s) = tag {
                            width = *s;
                        }
                    }
                    "Y" => {
                        if let nbt::Tag::Short(s) = tag {
                            height = *s;
                        }
                    }
                    "Z" => {
                        if let nbt::Tag::Short(s) = tag {
                            length = *s;
                        }
                    }
                    "BlockArray" => {
                        if let nbt::Tag::ByteArray(b) = tag {
                            blocks = b.iter().map(|by| *by as u8).collect();
                        }
                    }
                    "Spawn" => {
                        if let nbt::Tag::Compound(sv) = tag {
                            for (skey, stag) in sv {
                                match skey.as_str() {
                                    "X" => {
                                        if let nbt::Tag::Short(s) = stag {
                                            spawn.0 = *s;
                                        }
                                    }
                                    "Y" => {
                                        if let nbt::Tag::Short(s) = stag {
                                            spawn.1 = *s;
                                        }
                                    }
                                    "Z" => {
                                        if let nbt::Tag::Short(s) = stag {
                                            spawn.2 = *s;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {
                        println!("Name: {} - Tag: {:?}", key, tag);
                    }
                }
            }
        }

        // TODO(nv): check socket code in classicube
        // 256 - 64 - 256 only half of the world is sent
        // 128 - 64 - 128 works fine

        Ok(World {
            width,
            height,
            length,
            blocks,
            spawn,
        })
    }

    pub fn save_world<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        // Create file
        let f = File::create(path)?;
        let w = BufWriter::new(f);
        let mut gz = write::GzEncoder::new(w, Default::default());

        // TODO(nv): probably reuse loaded nbt from cw file -- save inside world

        // Create nbt
        let mut m = HashMap::<String, nbt::Tag>::new();
        m.insert("FormatVersion".into(), nbt::Tag::Byte(1));
        m.insert("X".into(), nbt::Tag::Short(self.width));
        m.insert("Y".into(), nbt::Tag::Short(self.height));
        m.insert("Z".into(), nbt::Tag::Short(self.length));
        m.insert(
            "BlockArray".into(),
            nbt::Tag::ByteArray(self.blocks.iter().map(|b| *b as i8).collect()),
        );

        let mut sm = HashMap::<String, nbt::Tag>::new();
        sm.insert("X".into(), nbt::Tag::Short(self.spawn.0));
        sm.insert("Y".into(), nbt::Tag::Short(self.spawn.1));
        sm.insert("Z".into(), nbt::Tag::Short(self.spawn.2));

        m.insert("Spawn".into(), nbt::Tag::Compound(sm));

        let nbt = NBT::new("ClassicWorld", nbt::Tag::Compound(m));

        // Write into file
        nbt.write(&mut gz)?;
        Ok(())
    }

    pub fn spawning_point(&mut self) -> (i16, i16, i16) {
        // Convert world coords to player's
        let world_x = (self.spawn.0 as f64 * 32.0) as i16;
        let world_y = (self.spawn.1 as f64 * 32.0) as i16;
        let world_z = (self.spawn.2 as f64 * 32.0) as i16;
        (world_x, world_y, world_z)
    }

    pub fn gzip_world(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut gzipper = write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let world_size = self.width as i32 * self.height as i32 * self.length as i32;
        gzipper.write(&world_size.to_be_bytes())?; // world size
        gzipper.write(&self.blocks)?;

        Ok(gzipper.finish()?)
    }

    // TODO(nv): move outside of world?
    pub fn send_world<W: Write>(&mut self, writer: &mut W) -> anyhow::Result<()> {
        // Init level transmition
        packets::level_init(writer, ServerPacket::LevelInit)?;

        // Algorithm to send bytes in chunk
        let gblocks = self.gzip_world()?;
        let total_bytes = gblocks.len();
        let mut current_bytes = 0;
        while current_bytes < total_bytes {
            let remaining_bytes = total_bytes - current_bytes;
            let count = if remaining_bytes >= 1024 {
                1024
            } else {
                remaining_bytes
            };

            // Just hack - predicted percentage
            let tmp_curr_bytes = current_bytes + count;
            let percentage = ((tmp_curr_bytes as f32 / total_bytes as f32) * 100.0) as u8;

            packets::level_chunk_data(
                writer,
                ServerPacket::LevelData {
                    length: count as i16,
                    data: &gblocks[current_bytes..current_bytes + count],
                    percentage,
                },
            )?;

            current_bytes += count;
        }

        // Finalize transmition
        packets::level_finalize(
            writer,
            ServerPacket::LevelFinal {
                width: self.width,
                height: self.height,
                length: self.length,
            },
        )?;

        Ok(())
    }
}
