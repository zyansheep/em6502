#![allow(unused)]
#![feature(bigint_helper_methods)]

mod rom;
mod instructions;

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
    /// Latch register for temporary holding
    latch: u8,
}

pub struct Memory {
    /// 2KB of internal RAM
    ram: [u8; 0x0800],
    /// Picture Processing Unit Registers
    ppu: [u8; 0x0008],
    /// Audio Processing Unit Registers
    apu: [u8; 0x0018],
    /// Testing registers
    test: [u8; 0x0008],
    /// Data mapped from cartridge, may be writable.
    cartridge: [u8; 0xBFE0],
}
impl Memory {
    fn new() -> Self {
        Self {
            ram: [0u8; 0x0800],
            ppu: [0u8; 0x0008],
            apu: [0u8; 0x0018],
            test: [0u8; 0x0008],
            cartridge: [0u8; 0xBFE0],
        }
    }
    fn mem_map(&mut self, addr: u16) -> &mut u8 {
        let idx = addr as usize;
        match addr {
            /// Access internal RAM (is mirrored 4 times, total size 0x0800)
            0x0000..=0x1FFF => &mut self.ram[idx % 0x0800],
            /// Access the PPU, repeats every 8 bytes until 0x1FF8
            0x2000..=0x3FFF => &mut self.ppu[idx % 0x0008],
            0x4000..=0x4017 => &mut self.apu[idx],
            0x4018..=0x401F => &mut self.test[idx],
            0x4020..=0xFFFF => &mut self.cartridge[idx],
        }
    }
    pub fn read(&mut self, addr: u16) -> u8 {
        self.mem_map(addr).clone()
    }
    pub fn write(&mut self, addr: u16, val: u8) {
        *self.mem_map(addr) = val;
    }
}
#[derive(Debug, Default)]
struct MemoryBus {
    /// Lower 8 bits of address
    low: u8,
    /// Upper 8 bits of address
    high: u8,
    /// In / Out 8 bits
    wire: u8,
}
impl MemoryBus {
    fn set(&mut self, addr: u16) {
        let bytes = addr.to_le_bytes();
        self.low = bytes[0];
        self.high = bytes[1];
    }
    fn read(&mut self, addr: u16) -> u8 {
        self.set(addr);
        self.wire
    }
}

/// Derived from: https://www.nesdev.org/wiki/CPU_memory_map
pub struct State {
    mem: Memory,
    bus: MemoryBus,
    cpu: CpuState,
    /// Current instruction that may be executing
    instr_indx: usize,
    /// Current cycle of the current instruction executing
    cycle_idx: usize,
    /// State of instruction executing
    op_state: OpState,
}

impl State {
    fn new() -> Self {
        State {
            mem: Memory::new(),
            bus: Default::default(),
            cpu: Default::default(),
            instr_indx: 0,
            cycle_idx: 0,
            op_state: Default::default(),
        }
    }
    fn read(&mut self) {
        self.bus.wire = self.mem.read(u16::from_be_bytes([self.bus.high, self.bus.low]));
    }
    fn write(&mut self) {
        self.mem.write(u16::from_be_bytes([self.bus.high, self.bus.low]), self.bus.wire);
    }
    /// Run a single CPU cycle
    fn step(&mut self, instr_set: [Vec<fn(&mut State)>; 256]) {
        match self.op_state {
            // If fetching op_state, get instr_idx, access from memory and set active op_state
            OpState::Fetching => {
                self.bus.set(self.cpu.pc); self.read();
                self.instr_indx = self.bus.wire as usize;
                self.cpu.pc += 1;
                self.op_state = OpState::Active;
            }
            /// If Page cross, increment high page and set address
            OpState::PageCross => {
                self.bus.high += 1;
                self.read();
            }
            // If not fetching, execute active instruction
            OpState::Active => {
                let instr_set = &instr_set[self.instr_indx];
                instr_set[self.cycle_idx](self);
                // If no more idx, reset OpState
                if instr_set.len() == self.cycle_idx {
                    self.op_state = OpState::Fetching;
                }
            }
        }
    }
}

/// State to keep track of temporary values used between cycles while executing an instruction.
#[derive(Debug, Default)]
pub enum OpState {
    /// Waiting for new instruction to be fetched
    #[default]
    Fetching,
    /// No state
    Active,
    /// There was a page cross when reading, increment high address before running next instruction.
    PageCross,
}