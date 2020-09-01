use crate::packets::{self, ServerPacket};
use flate2::write::GzEncoder;
use std::io::Write;

pub struct World {
    pub width: i16,
    pub height: i16,
    pub length: i16,
    pub blocks: Vec<u8>,
}

impl World {
    pub fn new(width: i16, height: i16, length: i16) -> Self {
        let count = width * height * length;
        let blocks = vec![0x00u8; count as usize]; // fill with air
        let mut world = World {
            width,
            height,
            length,
            blocks,
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
        return (x + self.width * (z + self.length * y)) as usize;
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

    pub fn spawning_center(&mut self) -> (i16, i16, i16) {
        // Convert world coords to player's
        let world_x = ((self.width / 2) as f64 * 32.0) as i16;
        let world_y = ((self.height / 2) as f64 * 32.0) as i16;
        let world_z = ((self.length / 2) as f64 * 32.0) as i16;
        (world_x, world_y, world_z)
    }

    pub fn gzip_world(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut gzipper = GzEncoder::new(Vec::new(), flate2::Compression::default());
        let world_size = (self.width * self.height * self.length) as i32;
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
        let mut percentage = 0u8;
        while current_bytes < total_bytes {
            let remaining_bytes = total_bytes - current_bytes;
            let count = if remaining_bytes >= 1024 {
                1024
            } else {
                remaining_bytes
            };

            packets::level_chunk_data(
                writer,
                ServerPacket::LevelData {
                    length: count as i16,
                    data: &gblocks[current_bytes..count],
                    percentage,
                },
            )?;

            current_bytes += count;

            percentage = ((current_bytes as f32 / total_bytes as f32) * 100.0) as u8;
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
