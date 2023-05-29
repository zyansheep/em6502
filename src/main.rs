#![allow(unused)]
#![allow(non_camel_case_types)]
#![feature(bigint_helper_methods)]
#![feature(generic_arg_infer)]
#![feature(generic_const_exprs)]
#![feature(const_mut_refs)]
mod rom;
mod instructions;
use instructions::INSTR_SET;

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

    let rom = rom::load_rom(&path, &mut state)?;

    state.init();
    println!("{:?}", &state.mem.cartridge[state.mem.cartridge.len()-20..]);
    println!("Starting CPU: {:?}", state.cpu);
    while state.step(INSTR_SET) {
        println!("cpu state: {:?}", state.cpu);
    }

    println!("CPU: {:#?}, op_status: {:?}, instr_idx: 0x{:x?}", state.cpu, state.op_state, state.instr_indx);
    
    let unused_bytes = state.mem.cartridge.len() - (rom.len() - 0x10);
    println!("Executing byte 0x{:x?} in ROM", state.cpu.pc - 0x4020 - (unused_bytes as u16));
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
impl CpuState {
    fn pc_get(&mut self) -> [u8; 2] { self.pc.to_le_bytes() }
    fn pc_set(&mut self, pc: [u8; 2]) { self.pc = u16::from_le_bytes(pc) }
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
            0x2000..=0x3FFF => &mut self.ppu[(idx - 0x2000) % 0x0008],
            0x4000..=0x4017 => &mut self.apu[idx - 0x4000],
            0x4018..=0x401F => &mut self.test[idx - 4018],
            0x4020..=0xFFFF => &mut self.cartridge[idx - 0x4020],
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
    fn init(&mut self) {
        let low = self.read_at(0xFFFC);
        let high = self.read_at(0xFFFD);
        println!("found: {:?}", (low, high));
        println!("{:?}", &self.mem.cartridge[self.mem.cartridge.len()-20..]);
        self.cpu.pc_set([low, high]);
    }
    fn read(&mut self) {
        self.bus.wire = self.mem.read(u16::from_be_bytes([self.bus.high, self.bus.low]));
    }
    fn read_at(&mut self, addr: u16) -> u8 {
        self.bus.set(addr);
        self.read();
        self.bus.wire
    }
    fn write(&mut self) {
        self.mem.write(u16::from_be_bytes([self.bus.high, self.bus.low]), self.bus.wire);
    }
    /// Run a single CPU cycle
    fn step(&mut self, instr_set: [&'static [fn(&mut State)]; 256]) -> bool {
        if self.op_state.contains(OpState::Active) {
            if self.op_state.contains(OpState::PageCross | OpState::Branching) {
                // Deal with branching before page cross
                if self.op_state.contains(OpState::Branching) {
                    self.op_state.remove(OpState::PageCross);
                } else { // Deal with page cross
                    self.bus.high += 1;
                    self.read();
                    self.op_state.remove(OpState::PageCross);
                }
            } else {
                let instr_set = instr_set[self.instr_indx];
                if instr_set.len() == 0 { return false }
                instr_set[self.cycle_idx](self);
                // If no more idx, reset OpState
                if instr_set.len() == self.cycle_idx {
                    self.op_state.remove(OpState::Active);
                }
            }
        } else {
            // Read new instruction
            self.instr_indx = self.read_at(self.cpu.pc) as usize;
            self.cpu.pc += 1;
            self.op_state.insert(OpState::Active);
        }
        false
    }
}


bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    struct OpState: u8 {
        const Active    = 0b0000_0001;
        const PageCross = 0b0000_0010;
        const Branching = 0b0000_0100;
    }
}

/* /// State to keep track of temporary values used between cycles while executing an instruction.
#[derive(Debug, Default)]
pub enum OpState {
    /// Waiting for new instruction to be fetched
    #[default]
    Fetching,
    /// No state
    Active,
    /// There was a page cross when reading, increment high address before running next instruction.
    PageCross,
    /// A branch was triggered. u8 indicates 
    Branching(u8),
} */