#![forbid(unsafe_code)]

//! iNES 1.0 and NES 2.0 image parsing.
//!
//! The parser preserves raw header bytes and ROM payloads. It does not attempt
//! to repair malformed dumps or infer mapper behavior.

use core::fmt;

const HEADER_LEN: usize = 16;
const TRAINER_LEN: usize = 512;
const MAGIC: [u8; 4] = *b"NES\x1A";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomFormat {
    INes,
    Nes2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TvSystem {
    Ntsc,
    Pal,
    MultiRegion,
    Dendy,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RomHeader {
    pub format: RomFormat,
    pub mapper: u16,
    pub submapper: u8,
    pub prg_rom_size: usize,
    pub chr_rom_size: usize,
    pub prg_ram_size: usize,
    pub prg_nvram_size: usize,
    pub chr_ram_size: usize,
    pub chr_nvram_size: usize,
    pub mirroring: Mirroring,
    pub battery_backed: bool,
    pub has_trainer: bool,
    pub tv_system: TvSystem,
    pub console_type: u8,
    pub raw: [u8; HEADER_LEN],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomImage {
    pub header: RomHeader,
    pub trainer: Option<[u8; TRAINER_LEN]>,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    /// Bytes after the declared PRG and CHR payloads.
    pub trailing_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomError {
    HeaderTooShort { actual: usize },
    InvalidMagic,
    UnsupportedConsoleType { console_type: u8 },
    InvalidNes2SizeEncoding,
    SizeOverflow,
    TruncatedTrainer { expected: usize, actual: usize },
    TruncatedPrgRom { expected: usize, actual: usize },
    TruncatedChrRom { expected: usize, actual: usize },
}

impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTooShort { actual } => write!(f, "ROM header is truncated: {actual}/16 bytes"),
            Self::InvalidMagic => write!(f, "invalid iNES magic; expected NES\\x1A"),
            Self::UnsupportedConsoleType { console_type } => write!(f, "unsupported NES 2.0 console type {console_type}"),
            Self::InvalidNes2SizeEncoding => write!(f, "invalid NES 2.0 ROM size encoding"),
            Self::SizeOverflow => write!(f, "ROM size does not fit this platform"),
            Self::TruncatedTrainer { expected, actual } => write!(f, "trainer is truncated: expected {expected} bytes, found {actual}"),
            Self::TruncatedPrgRom { expected, actual } => write!(f, "PRG ROM is truncated: expected {expected} bytes, found {actual}"),
            Self::TruncatedChrRom { expected, actual } => write!(f, "CHR ROM is truncated: expected {expected} bytes, found {actual}"),
        }
    }
}

impl std::error::Error for RomError {}

fn nes2_rom_size(lsb: u8, ms_nibble: u8, unit: usize) -> Result<usize, RomError> {
    if ms_nibble != 0x0F {
        let count = (usize::from(ms_nibble) << 8) | usize::from(lsb);
        count.checked_mul(unit).ok_or(RomError::SizeOverflow)
    } else {
        let exponent = u32::from(lsb >> 2);
        let multiplier = u32::from(lsb & 0x03) * 2 + 1;
        1usize.checked_shl(exponent)
            .and_then(|base| base.checked_mul(multiplier as usize))
            .ok_or(RomError::SizeOverflow)
    }
}

fn ram_size(shift: u8) -> Result<usize, RomError> {
    if shift == 0 { return Ok(0); }
    64usize.checked_shl(u32::from(shift)).ok_or(RomError::SizeOverflow)
}

impl RomImage {
    pub fn parse(data: &[u8]) -> Result<Self, RomError> {
        if data.len() < HEADER_LEN {
            return Err(RomError::HeaderTooShort { actual: data.len() });
        }
        if data[0..4] != MAGIC {
            return Err(RomError::InvalidMagic);
        }
        let mut raw = [0u8; HEADER_LEN];
        raw.copy_from_slice(&data[..HEADER_LEN]);

        let nes2 = data[7] & 0x0C == 0x08;
        let format = if nes2 { RomFormat::Nes2 } else { RomFormat::INes };
        let console_type = data[7] & 0x03;
        if console_type != 0 {
            return Err(RomError::UnsupportedConsoleType { console_type });
        }

        let mapper = (data[6] >> 4) as u16
            | (data[7] & 0xF0) as u16
            | if nes2 { ((data[8] & 0x0F) as u16) << 8 } else { 0 };
        let submapper = if nes2 { data[8] >> 4 } else { 0 };

        let (prg_rom_size, chr_rom_size) = if nes2 {
            (
                nes2_rom_size(data[4], data[9] & 0x0F, 16 * 1024)?,
                nes2_rom_size(data[5], data[9] >> 4, 8 * 1024)?,
            )
        } else {
            (
                usize::from(data[4]).checked_mul(16 * 1024).ok_or(RomError::SizeOverflow)?,
                usize::from(data[5]).checked_mul(8 * 1024).ok_or(RomError::SizeOverflow)?,
            )
        };

        let (prg_ram_size, prg_nvram_size, chr_ram_size, chr_nvram_size, tv_system) = if nes2 {
            (
                ram_size(data[10] & 0x0F)?,
                ram_size(data[10] >> 4)?,
                ram_size(data[11] & 0x0F)?,
                ram_size(data[11] >> 4)?,
                match data[12] & 0x03 {
                    0 => TvSystem::Ntsc,
                    1 => TvSystem::Pal,
                    2 => TvSystem::MultiRegion,
                    _ => TvSystem::Dendy,
                },
            )
        } else {
            let legacy_ram = if data[8] == 0 { 8 * 1024 } else { usize::from(data[8]) * 8 * 1024 };
            (
                legacy_ram,
                0,
                if chr_rom_size == 0 { 8 * 1024 } else { 0 },
                0,
                if data[9] & 1 != 0 { TvSystem::Pal } else { TvSystem::Ntsc },
            )
        };

        let mirroring = if data[6] & 0x08 != 0 {
            Mirroring::FourScreen
        } else if data[6] & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let header = RomHeader {
            format, mapper, submapper, prg_rom_size, chr_rom_size,
            prg_ram_size, prg_nvram_size, chr_ram_size, chr_nvram_size,
            mirroring, battery_backed: data[6] & 0x02 != 0,
            has_trainer: data[6] & 0x04 != 0, tv_system, console_type, raw,
        };

        let mut cursor = HEADER_LEN;
        let trainer = if header.has_trainer {
            let bytes = data.get(cursor..cursor + TRAINER_LEN).ok_or_else(|| RomError::TruncatedTrainer {
                expected: TRAINER_LEN,
                actual: data.len().saturating_sub(cursor),
            })?;
            let mut trainer = [0u8; TRAINER_LEN];
            trainer.copy_from_slice(bytes);
            cursor += TRAINER_LEN;
            Some(trainer)
        } else { None };

        let prg = data.get(cursor..cursor.saturating_add(prg_rom_size)).ok_or_else(|| RomError::TruncatedPrgRom {
            expected: prg_rom_size,
            actual: data.len().saturating_sub(cursor),
        })?;
        let prg_rom = prg.to_vec();
        cursor += prg_rom_size;

        let chr = data.get(cursor..cursor.saturating_add(chr_rom_size)).ok_or_else(|| RomError::TruncatedChrRom {
            expected: chr_rom_size,
            actual: data.len().saturating_sub(cursor),
        })?;
        let chr_rom = chr.to_vec();
        cursor += chr_rom_size;

        Ok(Self {
            header, trainer, prg_rom, chr_rom,
            trailing_data: data[cursor..].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ines(prg_banks: u8, chr_banks: u8) -> Vec<u8> {
        let mut data = vec![0; HEADER_LEN];
        data[..4].copy_from_slice(&MAGIC);
        data[4] = prg_banks;
        data[5] = chr_banks;
        data
    }

    #[test]
    fn parses_ines_prg_chr_and_mapper() {
        let mut data = ines(1, 1);
        data[6] = 0x21; // mapper low nibble 2, vertical mirroring
        data[7] = 0x30; // mapper high nibble 3
        data.extend(vec![0xAA; 16 * 1024]);
        data.extend(vec![0x55; 8 * 1024]);
        let rom = RomImage::parse(&data).unwrap();
        assert_eq!(rom.header.format, RomFormat::INes);
        assert_eq!(rom.header.mapper, 0x32);
        assert_eq!(rom.header.mirroring, Mirroring::Vertical);
        assert_eq!(rom.prg_rom.len(), 16 * 1024);
        assert_eq!(rom.chr_rom.len(), 8 * 1024);
    }

    #[test]
    fn parses_trainer_before_prg_rom() {
        let mut data = ines(1, 0);
        data[6] = 0x04;
        data.extend(vec![0x77; TRAINER_LEN]);
        data.extend(vec![0xAA; 16 * 1024]);
        let rom = RomImage::parse(&data).unwrap();
        assert_eq!(rom.trainer.unwrap()[0], 0x77);
        assert_eq!(rom.prg_rom[0], 0xAA);
        assert_eq!(rom.header.chr_ram_size, 8 * 1024);
    }

    #[test]
    fn parses_nes2_extended_mapper_and_submapper() {
        let mut data = ines(1, 0);
        data[6] = 0x10; // mapper low nibble 1
        data[7] = 0x08;
        data[8] = 0xA5; // submapper 10, mapper extension 5
        data.extend(vec![0xEA; 16 * 1024]);
        let rom = RomImage::parse(&data).unwrap();
        assert_eq!(rom.header.format, RomFormat::Nes2);
        assert_eq!(rom.header.mapper, 0x501);
        assert_eq!(rom.header.submapper, 10);
    }

    #[test]
    fn rejects_invalid_and_truncated_images() {
        assert_eq!(RomImage::parse(b"not a rom"), Err(RomError::HeaderTooShort { actual: 9 }));
        let mut data = ines(1, 0);
        assert_eq!(RomImage::parse(&data), Err(RomError::TruncatedPrgRom { expected: 16 * 1024, actual: 0 }));
        data[0] = 0;
        assert_eq!(RomImage::parse(&data), Err(RomError::InvalidMagic));
    }

    #[test]
    fn preserves_trailing_data() {
        let mut data = ines(1, 0);
        data.extend(vec![0; 16 * 1024]);
        data.extend_from_slice(b"TAIL");
        let rom = RomImage::parse(&data).unwrap();
        assert_eq!(rom.trailing_data, b"TAIL");
    }
}
