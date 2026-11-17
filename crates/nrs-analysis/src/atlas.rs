use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use png::{BitDepth, ColorType, Encoder};

use super::ChrTile;

const COLUMNS: usize = 16;
const SCALE: usize = 4;
const PADDING: usize = 2;
const TILE_SIZE: usize = 8;
const CELL_SIZE: usize = TILE_SIZE * SCALE + PADDING * 2;

pub fn write_chr_atlas_png(
    tiles: &[ChrTile],
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn Error>> {
    let rows = ((tiles.len() + COLUMNS - 1) / COLUMNS).max(1);

    let width = COLUMNS * CELL_SIZE;
    let height = rows * CELL_SIZE;

    let mut pixels = vec![24u8; width * height * 3];

    let palette = [0u8, 85, 170, 255];

    for (tile_index, tile) in tiles.iter().enumerate() {
        let tile_x = (tile_index % COLUMNS) * CELL_SIZE;
        let tile_y = (tile_index / COLUMNS) * CELL_SIZE;

        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let value = tile.pixels[y * TILE_SIZE + x] as usize;
                let shade = palette[value.min(3)];

                for sy in 0..SCALE {
                    for sx in 0..SCALE {
                        let px = tile_x + PADDING + x * SCALE + sx;
                        let py = tile_y + PADDING + y * SCALE + sy;

                        let offset = (py * width + px) * 3;

                        pixels[offset] = shade;
                        pixels[offset + 1] = shade;
                        pixels[offset + 2] = shade;
                    }
                }
            }
        }
    }

    let file = File::create(path)?;
    let writer = BufWriter::new(file);

    let mut encoder = Encoder::new(writer, width as u32, height as u32);
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Eight);

    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(&pixels)?;

    Ok(())
}
