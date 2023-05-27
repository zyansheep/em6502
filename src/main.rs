#![allow(unused)]

mod rom;

use std::{path::PathBuf, io::{self, Read}, fs};

use clap::{Parser};
use rom::{ROMError};
use thiserror::Error;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// Required binary path to run
    bin_path: PathBuf,
}

#[derive(Error, Debug)]
enum EmulatorError {
    #[error("invalid rom format: {0}")]
    ROMError(#[from] ROMError),
}

fn main() -> Result<(), EmulatorError> {
    let args = Arguments::parse();

    let path = args.bin_path;

    println!("Loading binary: {:?}", path);

    let mut state = State::new();

    if let Err(err) = rom::load_rom(&path, &mut state) {
        println!("failed to load rom: {err}");
    }


    Ok(())
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    struct CpuFlags: u8 {
        const Negative = 0b10000000;
        const Overflow = 0b01000000;
        const Decimal  = 0b00001000;
        const Zero     = 0b00000010;
        const Carry    = 0b00000001;
        const InterruptDisable = 0b00000100;
    }
}

/// Derived from: https://www.nesdev.org/wiki/CPU_registers and https://www.nesdev.org/wiki/Status_flags
#[derive(Default, Debug)]
struct CpuState {
    /// Accumulator Register
    a: u8,
    /// X Index Register
    x: u8,
    /// Y Index Register
    y: u8,
    /// Status Flags
    flags: CpuFlags,
    /// Program Counter
    pc: u16,
    /// Stack Pointer
    sp: u8,
}

/// Derived from: https://www.nesdev.org/wiki/CPU_memory_map
pub struct State {
    /// 2KB of internal RAM
    ram: [u8; 0x0800],
    /// Picture Processing Unit Registers
    ppu: [u8; 0x0008],
    /// Audio Processing Unit Registers
    apu: [u8; 0x0018],
    /// Testing registers
    test: [u8; 0x0008],
    cartridge: [u8; 0xBFE0],
    /// CPU Registers
    cpu: CpuState,
}

impl State {
    fn new() -> Self {
        State {
            mem: [0u8; u16::MAX as usize],
            cpu: Default::default(),
        }
    }
    fn mem(&mut self, addr: u16) -> &mut u8 {
        let idx = addr as usize;
        &mut match addr {
            /// Access internal RAM (is mirrored 4 times, total size 0x0800)
            0x0000..=0x1FFF => self.ram[idx % 0x0800],
            /// Access the PPU, repeats every 8 bytes until 0x1FF8
            0x2000..=0x1FF8 => self.ppu[idx % 0x0008],
            0x4000..=0x4017 => self.apu[idx],
            0x4018..=0x401F => self.test[idx],
            0x4020..=0xFFFF => self.cartridge[idx],
        }
    }
    pub fn mem_get(&mut self, addr: u16) -> u8 {
        self.mem(addr).clone()
    }
    pub fn mem_set(&mut self, addr: u16, val: u8) {
        *self.mem(addr) = val;
    }
    /// Run the CPU
    fn step(&mut self, instr_set: [fn(&mut State); 256]) {
        // Get instruction data from memory
        let instr_idx = self.mem_get(self.cpu.pc) as usize;
        // Execute corresponding instruction
        instr_set[instr_idx](self);
    }
}

/// Structures that extract data from state.
trait AddressingMode<DataType> {
    fn load(state: &mut State) -> DataType;
}

/// Any object that implements both an InstructionType and an AddressingMode that supports the InstructionType's input.
trait Instruction: InstructionType + AddressingMode<<Self as InstructionType>::Data> {
    fn e(state: &mut State) {
        let data = <Self as AddressingMode<<Self as InstructionType>::Data>>::load(state);
        <Self as InstructionType>::execute(state, data);
    }
}


/// Some type of instruction, irrespective of the addressing model.
trait InstructionType {
    type Data;
    fn execute(state: &mut State, data: Self::Data);
}
/// Load to A
struct LDA;
impl InstructionType for LDA {
    type Data = u8;
    fn execute(state: &mut State, data: Self::Data) {
        state.cpu.a = data;
    }
}
/// Transfer A to X
struct TAX;
/// Transfer X to A
struct TXA;
struct PHA;
struct PLA;
