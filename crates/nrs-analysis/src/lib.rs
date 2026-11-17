mod atlas;
pub use atlas::write_chr_atlas_png;

use nrs_rom::{Mirroring, RomFormat, RomImage, TvSystem};

pub mod disasm;
pub mod coverage;
pub mod code_data;

pub use disasm::{Disassembly, Instruction, disassemble_nrom};
pub use coverage::{CoverageRange, CoverageReport, analyze_coverage, format_coverage_report};
pub use code_data::{ByteKind, CodeDataReport, CodeDataSegment, analyze_code_data, format_code_data_report};

pub mod labels;
pub mod xref;

pub use labels::{Label, LabelKind, format_listing_with_labels, generate_labels};

pub use xref::{CrossReference, ReferenceKind, build_cross_references, format_cross_references};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptVectors {
    pub nmi: u16,
    pub reset: u16,
    pub irq: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrTile {
    pub pixels: [u8; 64],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomAnalysis {
    pub format: RomFormat,
    pub mapper: u16,
    pub submapper: u8,
    pub prg_size: usize,
    pub chr_size: usize,
    pub mirroring: Mirroring,
    pub tv_system: TvSystem,
    pub has_trainer: bool,
    pub battery_backed: bool,
    pub trailing_size: usize,
    pub vectors: Option<InterruptVectors>,
    pub chr_tile_count: usize,
}

impl RomAnalysis {
    pub fn inspect(rom: &RomImage) -> Self {
        let vectors = read_vectors(&rom.prg_rom);

        Self {
            format: rom.header.format,
            mapper: rom.header.mapper,
            submapper: rom.header.submapper,
            prg_size: rom.prg_rom.len(),
            chr_size: rom.chr_rom.len(),
            mirroring: rom.header.mirroring,
            tv_system: rom.header.tv_system,
            has_trainer: rom.trainer.is_some(),
            battery_backed: rom.header.battery_backed,
            trailing_size: rom.trailing_data.len(),
            vectors,
            chr_tile_count: rom.chr_rom.len() / 16,
        }
    }
}

pub fn read_vectors(prg: &[u8]) -> Option<InterruptVectors> {
    if prg.len() < 6 {
        return None;
    }

    let start = prg.len() - 6;

    let read_word = |offset: usize| -> u16 {
        u16::from_le_bytes([prg[start + offset], prg[start + offset + 1]])
    };

    Some(InterruptVectors {
        nmi: read_word(0),
        reset: read_word(2),
        irq: read_word(4),
    })
}

/// Decodes NES 2bpp CHR data into 8x8 tiles.
/// Each pixel contains a palette index from 0 to 3.
pub fn decode_chr(chr: &[u8]) -> Vec<ChrTile> {
    let tile_count = chr.len() / 16;
    let mut tiles = Vec::with_capacity(tile_count);

    for tile_index in 0..tile_count {
        let base = tile_index * 16;
        let mut pixels = [0u8; 64];

        for y in 0..8 {
            let plane_0 = chr[base + y];
            let plane_1 = chr[base + y + 8];

            for x in 0..8 {
                let bit = 7 - x;
                let low = (plane_0 >> bit) & 1;
                let high = (plane_1 >> bit) & 1;

                pixels[y * 8 + x] = low | (high << 1);
            }
        }

        tiles.push(ChrTile { pixels });
    }

    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nes_interrupt_vectors_little_endian() {
        let prg = [0x00, 0x80, 0x34, 0xC0, 0x78, 0xE0];

        let vectors = read_vectors(&prg).unwrap();

        assert_eq!(vectors.nmi, 0x8000);
        assert_eq!(vectors.reset, 0xC034);
        assert_eq!(vectors.irq, 0xE078);
    }

    #[test]
    fn rejects_prg_without_complete_vectors() {
        assert_eq!(read_vectors(&[0; 5]), None);
    }

    #[test]
    fn decodes_chr_tile_bitplanes() {
        let mut chr = [0u8; 16];

        chr[0] = 0b1000_0000;
        chr[8] = 0b0100_0000;

        let tiles = decode_chr(&chr);

        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0].pixels[0], 1);
        assert_eq!(tiles[0].pixels[1], 2);
        assert_eq!(tiles[0].pixels[2], 0);
    }

    #[test]
    fn ignores_incomplete_chr_tile() {
        assert_eq!(decode_chr(&[0; 17]).len(), 1);
    }
}
