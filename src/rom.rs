use std::{path::{PathBuf, Path}, fs};

use bytes::Buf;
use thiserror::Error;

use crate::State;

#[derive(Error, Debug)]
pub enum ROMError {
    #[error("failed to load INES rom file: {0:?}")]
    IOError(#[from] std::io::Error),
    #[error("invalid magic value: {0:x?} .nes file should have magic bytes [4e, 45, 53, 1a] at the beginning.")]
    InvalidMagicValue([u8; 4]),
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct NESFlags67: u8 {
        /// Enables 4-screen display (VRAM)
        const FourScreen = 0b10000000;
        /// Mapper register translation code / CHR caching - 512-bytes at $7000-$71FF (stored before PRG data)
        const Trainer    = 0b01000000;
        /// Cartridge contains battery-backed PRG RAM ($6000-7FFF) or other persistent memory
        const BatteryRam = 0b00100000;
        /// Mirroring: 0: horizontal (vertical arrangement) (CIRAM A10 = PPU A11)
        ///            1: vertical (horizontal arrangement) (CIRAM A10 = PPU A10)
        const Mirroring  = 0b00010000;
        /// Enable NES2 backwards-compatible format of the NES format.
        const NES2Format = 0b00001000;
        const NES1Format = 0b00000100;
        /// PlayChoice-10 (8 KB of Hint Screen data stored after CHR data)
        const PlayChoice = 0b00000010;
        /// VS Unisystem
        const VSUnisys   = 0b00000001;
    }
}

pub fn load_rom(path: &Path, state: &mut State) -> Result<Vec<u8>, ROMError> {
    let file = fs::read(path)?;
    let mut nes = file.as_slice();

    /// Parse NES file format: https://www.nesdev.org/wiki/INES
    if &nes[0..4] == b"NES\x1a" {
        nes.advance(4);
        let program_size = nes.get_u8() as usize * 16384;
        let graphics_size = nes.get_u8() as usize * 8192;
        let flags67 = nes.get_u16();
        let mapper_upper = ((flags67 << 4) as u8) & 0b0000_1111; // Get upper mapper bits
        let mapper = ((flags67 as u8) & 0b1111_0000) | mapper_upper; // Join upper with lower bits
        let flags_upper = ((flags67 << 8) as u8) & 0b0000_1111;
        let flags = NESFlags67::from_bits_retain(((flags67 as u8) << 4) | flags_upper);

        println!("program_size: {program_size:?}, graphics_size: {graphics_size:?}");
        println!("mapper: {mapper:#b}, flags: {flags:#b}");

        // Not dealing with Trainer region for now...
        if flags.contains(NESFlags67::Trainer) {
            nes.advance(512);
        }

        // Copy ROM to cartridge ram.
        // It should be written so that it fits up to the very end of the address space
        let start = state.mem.cartridge.len()-nes.len();
        state.mem.cartridge[start..].copy_from_slice(&nes[..]);
        Ok(file)
    } else {
        // println!("{:x?} != {:x?}", &nes[0..4], b"NES\x1a");
        Err(ROMError::InvalidMagicValue((&nes[0..4]).try_into().unwrap()))
    }
}