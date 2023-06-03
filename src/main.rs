#![allow(unused)]
#![allow(non_camel_case_types)]
#![feature(bigint_helper_methods)]
#![feature(generic_arg_infer)]
#![feature(generic_const_exprs)]
#![feature(const_mut_refs)]
#![feature(adt_const_params)]
mod rom;
mod instructions;
mod cpu;
use bitflags::bitflags;
use instructions::{INSTR_SET, MathOp};
pub use cpu::*;
use rom::{ROMError};

use std::{path::PathBuf, io::{self, Read, Write}, fs};

use clap::{Parser};
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

    // println!("Loading binary: {:?}", path);

    let mut state = State::new();

    let rom = rom::load_rom(&path, &mut state)?;

    state.reset();
    // println!("Start: {state:?}");
    let unused_bytes = state.mem.cartridge.len() - (rom.len() - 0x10);
    // println!("Executing byte 0x{:x?} in ROM", state.cpu.pc - 0x4020 - (unused_bytes as u16) + 0x10);
    
    while state.step() {
        // println!("State: {state:?}");
        if state.instr_count > 9000 { println!("BROKE"); break; }
        // if state.cpu.pc == 0 { println!("reached end"); break }
        //println!("Executing instruction: `{:?}` at byte 0x{:x?} in ROM", INSTR_SET[state.instr_indx].0, byte_num);
    }

    println!("Final: {state:?}");

    let mut file = std::fs::File::create("testing.ram").unwrap();
    file.write_all(&state.mem.ram);

    Ok(())
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
    bytes_unused: u16,
}
impl Memory {
    fn new() -> Self {
        Self {
            ram: [0u8; 0x0800],
            ppu: [0u8; 0x0008],
            apu: [0u8; 0x0018],
            test: [0u8; 0x0008],
            cartridge: [0u8; 0xBFE0],
            bytes_unused: 0xBFE0,
        }
    }
    fn mem_map(&mut self, addr: u16) -> &mut u8 {
        let idx = addr as usize;
        match addr {
            /// Access internal RAM (is mirrored 4 times, total size 0x0800)
            0x0000..=0x1FFF => &mut self.ram[idx % 0x0800],
            /// Access the PPU, repeats every 8 bytes until 0x1FF8
            0x2000..=0x3FFF => {panic!("accessed PPU"); &mut self.ppu[(idx - 0x2000) % 0x0008]},
            0x4000..=0x4017 => {panic!("accessed APU"); &mut self.apu[idx - 0x4000]},
            0x4018..=0x401F =>{panic!("accessed APU"); &mut self.test[idx - 4018]},
            0x4020..=0xFFFF => &mut self.cartridge[idx - 0x4020],
        }
    }
    // Converts from Memory address to ROM address. Memory Addr may not be in rom.
    pub fn mem_to_rom(&self, addr: u16) -> Option<u16> {
        let rom_start = 0x4020u16 + self.bytes_unused;
        if (rom_start..=0xFFFFu16).contains(&addr) { Some((addr - rom_start) + 0x10) } else { None }
    }
    pub fn read(&mut self, addr: u16) -> u8 {
        let out = self.mem_map(addr).clone();
        // println!("READ: {addr:#06X?} = {out:#04X?} {}", self.mem_to_rom(addr).map_or(String::new(), |x|format!("({:#06X?})", x)));
        out
    }
    pub fn write(&mut self, addr: u16, val: u8) {
        // println!("WRITE: {addr:#06X?} = {val:#04X?} ({:?})", self.mem_to_rom(addr).map_or(format!("??"), |x|format!("{:#06X?}", x)));
        *self.mem_map(addr) = val;
    }
}

pub struct PPUState {
    
}

/// Derived from: https://www.nesdev.org/wiki/CPU_memory_map
pub struct State {
    mem: Memory,
    cpu: CPU,
    ppu: PPU,
    /// Current instruction that may be executing
    instr_indx: usize,
    /// Current cycle of the current instruction executing
    cycle_idx: usize,
    /// State of instruction executing
    op_state: OpState,
    cycle_count: usize,
    instr_count: usize,
    log: Logging,
}

#[derive(Debug, Default, Clone)]
struct Logging {
    opcode: u8,
    opcode_addr: u16,
    // operand if has one
    operand: Option<u8>,
    start_cycle: usize,
    start_cpu: CPU,
    // effective address if read/write memory
}
impl Logging {
    fn new_instr(state: &mut State, opcode: u8, opcode_addr: u16) {
        state.log = Logging {
            opcode, opcode_addr, start_cycle: state.cycle_count, start_cpu: state.cpu.clone(),
            ..Default::default()
        }
    }
    fn log(state: &mut State, instr_str: &str) {
        let cpu = &state.log.start_cpu;
        let cpu_str = format!("A:{:02X?}, X:{:02X?}, Y:{:02X?}, P:{:02X?}, SP:{:02X?}   CYC: {}", cpu.a, cpu.x, cpu.y, cpu.flags.bits(), cpu.sp, state.log.start_cycle);
        let main_str = format!("{:04X?}{}: {:02X?} {}. {}",
            state.log.opcode_addr,
            state.mem.mem_to_rom(state.log.opcode_addr).map_or(String::new(), |x|format!("({:#06X?})", x)),
            state.log.opcode, instr_str,
            state.log.operand.map_or("??".to_owned(), |x|format!("{:02X?}", x)),
        );
        println!("{:<30} {}", main_str, cpu_str);
    }
    /* fn log_mem_op(state: &mut State, operand: u8) {
        state.log.last_mem = u16::from_le_bytes([state.cpu.io.low, state.cpu.io.high]);
        state.log.operand = Some(operand);
    } */
}

impl State {
    fn new() -> Self {
        State {
            mem: Memory::new(),
            cpu: Default::default(),
            ppu: Default::default(),
            instr_indx: 0,
            cycle_idx: 0,
            cycle_count: 0,
            instr_count: 0,
            op_state: Default::default(),
            log: Default::default(),
        }
    }
    fn reset(&mut self) {
        self.instr_count = 0;
        self.log = Logging::default();
        let low = self.read_at(0xFFFC);
        let high = self.read_at(0xFFFD);
        self.cpu.pc = 0xC000;
        // self.cpu.pc_set([low, high]);
        self.cycle_count = 7;
        self.cpu.flags = CpuFlags::Unused | CpuFlags::InterruptDisable;
        self.cpu.sp = 0xFD;
        self.read();
    }
    fn read(&mut self) {
        self.cpu.io.wire = self.mem.read(u16::from_be_bytes([self.cpu.io.high, self.cpu.io.low]));
    }
    fn read_at(&mut self, addr: u16) -> u8 {
        self.cpu.io.set(addr);
        self.read();
        self.cpu.io.wire
    }
    fn write(&mut self) {
        self.mem.write(u16::from_be_bytes([self.cpu.io.high, self.cpu.io.low]), self.cpu.io.wire);
    }
    fn read_instr(&mut self) {
        if self.instr_count != 0 { Logging::log(self, INSTR_SET[self.instr_indx].0); }
        // Read new instruction
        self.instr_indx = self.read_at(self.cpu.pc) as usize;
        self.cycle_idx = 0;

        // logging
        Logging::new_instr(self, self.instr_indx as u8, self.cpu.pc);
        self.instr_count += 1;

        self.cpu.pc = self.cpu.pc.wrapping_add(1);
        self.op_state.insert(OpState::Active);
    }
    /// Run a single CPU cycle
    fn step(&mut self) -> bool {
        let old_cpu = self.cpu.clone();
        let old_op_state = self.op_state;
        
        // if self.cpu.flags.contains(CpuFlags::Break) | self.cpu.flags.contains(CpuFlags::InterruptDisable) { return false }
        
        // Deal with branching and page crosses
        if self.op_state.contains(OpState::Branching) {
            self.op_state.remove(OpState::Branching);
            self.op_state.remove(OpState::Active);
        } else if self.op_state.contains(OpState::PageCross) { // Deal with page cross
            self.cpu.io.high = self.cpu.io.high.wrapping_add(1);
            self.cpu.pc = self.cpu.pc.wrapping_add(0x0100);
            self.read();
            self.op_state.remove(OpState::PageCross);
        } else if self.op_state.contains(OpState::Active) {
            let instr_set = INSTR_SET[self.instr_indx].1;
            if instr_set.len() == 0 { Logging::log(self, INSTR_SET[self.instr_indx].0); return false }

            // Run op on state
            instr_set[self.cycle_idx](self);
            // If no more idx, reset OpState
            self.cycle_idx += 1;
            if instr_set.len() == self.cycle_idx {
                self.op_state.remove(OpState::Active);
            }
        } else {
            self.read_instr();
        }
        //old.cmp(&self.cpu);
        // if old_op_state != self.op_state { println!("OP_STATE: {:?} -> {:?}", old_op_state, self.op_state); }
        self.cycle_count += 1;
        true
    }
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("op_s", &self.op_state)
            .field("cc", &self.cycle_count)
            .field("cpu", &self.cpu)
            .finish()
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

#[derive(Default, Debug, Clone)]
struct PPU {
    io: PPUIO,
}

#[derive(Default, Debug, Clone)]
/// Manages the I/O state of the PPU
struct PPUIO {
    /// Control flags
    ctrl: PPUCtrl,
    /// Mask flags
    mask: PPUMask,
    /// Status flags
    status: u8,
    oam_addr: u8,
    oam_data: u8,
    scroll: u8,
    addr: u8,

    dma: u8,
}
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct PPUCtrl: u8 {
        /// Generate an NMI (Non-Maskable Interrupt) at the start of the vertical blanking interval (0: off; 1: on)
        const VBlankNMI      = 0b1000_0000;
        /// (0: read backdrop from EXT pins; 1: output color on EXT pins)
        const EXTCtrlSelect  = 0b0100_0000;
        /// Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
        const SpriteSize     = 0b0010_0000;
        /// Background Pattern Table Addr: 0=$0000, 1=$1000
        const BgPatTblAddr   = 0b0001_0000;
        /// Sprite pattern table address for 8x8 sprites (0: $0000; 1: $1000; ignored in 8x16 mode)
        const PatTblAddrType = 0b0000_1000;
        /// VRAM address increment per CPU read/write of PPUDATA (0: add 1, going across; 1: add 32, going down)
        const VRAMAddrInc    = 0b0000_0010;
        /// Base Nametable Addr: 0=$2000, 1=$2400, 2=$2800, 3=$2C00
        const BaseNameAddr = 0b0000_0011;
    }
}
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct PPUMask: u8 {
        /// Emphasize blue
        const EmphasizeBlue = 0b1000_0000;
        /// Emphasize green (red on PAL/Dendy)
        const EmphasizeGreen = 0b0100_0000;
        /// Emphasize red (green on PAL/Dendy)
        const EmphasizeRed = 0b0010_0000;
        /// Show sprites (0: hide, 1: show)
        const SpritesShow = 0b0001_0000;
        /// Show background (0: hide, 1: show)
        const BgShow = 0b0000_1000;
        /// Show sprites in leftmost 8 pixels of screen (0: hide, 1: show)
        const SpritesLeftmostShow = 0b0000_0100;
        /// Show background in leftmost 8 pixels of screen (0: hide, 1: show)
        const BgLeftmostShow = 0b0000_0010;
        /// Produce a greyscale display (0: normal color, 1: greyscale)
        const Greyscale = 0b0000_0001;
    }
}
