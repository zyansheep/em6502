//! Instructions are composed of "micro-operations" one of which runs each cycle that the instruction is active.
//! Each micro-op can in general do one memory operation and one ALU operation in parallel.
//! For instructions that don't modify memory, a memory operation is stil done because the min cycle count per instruction is 2
//! The micro-op cycle structure is outlined by this document: https://www.atarihq.com/danb/files/64doc.txt

mod table;
pub use table::INSTR_SET;
use std::{collections::HashMap, ops::Shl, io::Read};

use crate::{State, CpuFlags, OpState};

trait MathOp {
    fn exec(state: &mut State);
}

/// Performs ADD with Carry between A and Memory Bus.
struct ADC;
impl MathOp for ADC {
    fn exec(state: &mut State) {
        // Unsigned addition overflow changes the carry flag
        let (a_new, carry) = state.cpu.a.carrying_add(state.bus.wire, state.cpu.flags.contains(CpuFlags::Carry));
        state.cpu.flags.set(CpuFlags::Carry, carry);
        // Set overflow bit if most significant bit changed
        let negative = (a_new & 0b1000_0000) != 0;
        state.cpu.flags.set(CpuFlags::Negative, negative);
        state.cpu.flags.set(CpuFlags::Overflow, (state.cpu.a & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, a_new == 0);
        state.cpu.a = a_new;
        // Jump to next instruction
        state.cpu.pc += 1;
    }
}

/// Performs AND with A and Memory Bus and stores result in A.
struct AND;
impl MathOp for AND {
    fn exec(state: &mut State) {
        // Perform assign AND
        state.cpu.a &= state.bus.wire;
        state.cpu.flags.set(CpuFlags::Negative, (state.cpu.a & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, state.cpu.a == 0);
        // Jump to next instruction
        state.cpu.pc += 1;
    }
}

/// Performs AND with A and Memory Bus and sets flags.
struct BIT;
impl MathOp for BIT {
    fn exec(state: &mut State) {
        // Perform tmp AND.
        let res = state.cpu.a & state.bus.wire;
        state.cpu.flags.set(CpuFlags::Overflow, (state.bus.wire & 0b0100_0000) != 0);
        state.cpu.flags.set(CpuFlags::Negative, (res & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, res == 0);
        // Jump to next instruction
        state.cpu.pc += 1;
    }
}

/// Performs Arithmetic Shift Left on register A.
fn asl_a(state: &mut State) {
    let carry = (state.cpu.a & 0b1000_0000) != 0;
    state.cpu.a = state.cpu.a << 1;
    state.cpu.flags.set(CpuFlags::Carry, carry);
    state.cpu.flags.set(CpuFlags::Negative, state.cpu.a & 0b1000_0000 != 0);
    state.cpu.flags.set(CpuFlags::Zero, state.cpu.a == 0);
    // Jump to next instruction
    state.cpu.pc += 1;
}

struct ASL_MEM;
impl MathOp for ASL_MEM {
    fn exec(state: &mut State) {
        let carry = (state.bus.wire & 0b1000_0000) != 0;
        state.bus.wire = state.bus.wire << 1;
        state.cpu.flags.set(CpuFlags::Carry, carry);
        state.cpu.flags.set(CpuFlags::Negative, state.bus.wire & 0b1000_0000 != 0);
        state.cpu.flags.set(CpuFlags::Zero, state.bus.wire == 0);
        // Jump to next instruction
        state.cpu.pc += 1;
    }
}

/// Uses bus wire as low byte, and reads next byte in mem as high. sets PC counter accordingly.
/// Can be used for both absolute and indirect jumps
struct JMP;
impl MathOp for JMP {
    fn exec(state: &mut State) {
        state.cpu.pc = u16::from_le_bytes([state.bus.low, state.bus.high])
    }
}

/// Fetches immediate value and sets generates continuation

/// Memory Addressing Modes (Read)
/// Implied = [run<MATHOP>]
type InstrPipeline<const S: usize> = [fn(&mut State); S];
const fn implied<M: MathOp>() -> InstrPipeline<1> {
    [run::<M>]
}
const fn immediate<M: MathOp>() -> InstrPipeline<1> {
    [read_run::<PCRead, M>]
}
const fn relative<M: MathOp>() -> InstrPipeline<2> {
    [read_byte::<PCRead>, branch::<M>]
}
const fn indirect<M: MathOp>() -> InstrPipeline<4> {
    [read_byte::<PCRead>, read_addr::<PCRead>, read_to_latch, read_next_run::<JMP>]
}

const fn absolute<const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{2 + A}> {
    join([read_byte::<PCRead>, read_addr::<PCRead>], op)
}
const fn absolute_indexed<I: Register, const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{2 + A}> {
    join([read_byte::<PCRead>, add_index_low_read_high::<PCRead, I>], op)
}
const fn zeropage<const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{1 + A}> {
    join([read_byte::<PCRead>], op)
}
const fn zeropage_indexed<I: Register, const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{2 + A}> {
    join([read_byte::<PCRead>, read_add_index::<ZeroRead, I>], op)
}
const fn indexed_indirect<const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{4 + A}> {
    join([read_byte::<PCRead>, read_add_index::<ZeroRead, XIndex>, read_byte::<ZeroRead>, read_addr::<RegRead>], op)
}
const fn indirect_indexed<const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{3 + A}> {
    join([read_byte::<PCRead>, read_byte::<ZeroRead>, add_index_low_read_high::<RegRead, YIndex>], op)
}

const fn read_op<M: MathOp>() -> InstrPipeline<1> {
    [read_run::<RegRead, M>]
}
const fn write_op<M: MathOp>() -> InstrPipeline<1> {
    [write_run::<M>]
}
const fn rw_op<M: MathOp>() -> InstrPipeline<3> {
    [read_eff, rw_run::<M>, write_eff]
}

const fn join<const A: usize, const B: usize>(a: InstrPipeline<A>, b: InstrPipeline<B>) -> InstrPipeline<{A + B}> {
    let mut out: [fn(&mut State); {A + B}] = [read_byte::<PCRead>; {A + B}];
    let mut i = 0;
    while i < A {
        out[i] = a[i];
        i += 1;
    }
    while i < A + B {
        out[i] = b[i - A];
        i += 1;
    }
    out
}


/// Things that a given micro op should do pre and post-reading.
trait ReadType {
    fn pre(state: &mut State);
    fn post(state: &mut State);
    const PAGE_CROSS: bool;
}

/// Do nothing
struct RegRead;
impl ReadType for RegRead {
    fn pre(state: &mut State) {}
    fn post(state: &mut State) {}
    const PAGE_CROSS: bool = true;
}

/// Increment read from program counter and increment pc after reading.
struct PCRead;
impl ReadType for PCRead {
    fn pre(state: &mut State) {
        state.bus.set(state.cpu.pc);
    }
    fn post(state: &mut State) {
        state.cpu.pc += 1;
    }
    const PAGE_CROSS: bool = true;
}

/// Sets bus high byte to zero.
struct ZeroRead;
impl ReadType for ZeroRead {
    fn pre(state: &mut State) {
        state.bus.low = state.bus.wire;
        state.bus.high = 0;
    }
    fn post(state: &mut State) {}
    const PAGE_CROSS: bool = false;
}

trait Register {
    fn get(state: &State) -> u8;
}
struct XIndex;
impl Register for XIndex {
    fn get(state: &State) -> u8 { state.cpu.x }
}
struct YIndex;
impl Register for YIndex {
    fn get(state: &State) -> u8 { state.cpu.y }
}
struct PCL;
impl Register for PCL {
    fn get(state: &State) -> u8 { state.cpu.pc.to_le_bytes()[0] }
}
struct PCH;
impl Register for PCH {
    fn get(state: &State) -> u8 { state.cpu.pc.to_le_bytes()[1] }
}
struct BUS;
impl Register for BUS {
    fn get(state: &State) -> u8 { state.bus.wire }
}

/// Reads current byte to latch.
fn read_to_latch(state: &mut State) {
    state.read();
    state.cpu.latch = state.bus.wire;
}

/// sets higher byte from next memory location and sets lower byte from latch. Runs MathOp
fn read_next_run<M: MathOp>(state: &mut State) {
    state.bus.low += 1;
    state.read();
    state.bus.high = state.bus.wire;
    state.bus.low = state.cpu.latch;
    M::exec(state)
}

/// Reads a byte from address at program counter. Depending on R, can read from current location, program counter, or zeropage.
fn read_byte<R: ReadType>(state: &mut State) {
    R::pre(state);
    state.read();
    R::post(state)
}
/* fn read_byte_pc<PCRead>(state: &mut State) { read_byte::<PCRead>(state); }
fn read_byte_next(state: &mut State) { read_byte::<RegRead>(state); }
fn read_byte_zero(state: &mut State) { read_byte::<ZeroRead>(state); } */

/// Set bus address using previos read as low and next read as high.
fn read_addr<R: ReadType>(state: &mut State) {
    let low = state.bus.wire;
    
    state.bus.low += 1;
    R::pre(state);
    state.read();
    R::post(state);

    state.bus.low = low;
    state.bus.high = state.bus.wire;
}
fn read_addr_pc(state: &mut State) { read_addr::<PCRead>(state); }
fn read_addr_next(state: &mut State) { read_addr::<RegRead>(state); }
fn read_addr_zero(state: &mut State) { read_addr::<ZeroRead>(state); }

/// Increment previous read by index x or y, use as low byte. read new byte for high byte. Handle Page crossing
fn add_index_low_read_high<R: ReadType, I: Register>(state: &mut State) {
    /// Increment previous read by index.
    let (low, carry) = state.bus.wire.carrying_add(I::get(state), false);
    /// Read Byte
    R::pre(state);
    state.read();
    R::post(state);

    state.bus.low = low;
    state.bus.high = state.bus.wire;

    /// If high byte needs increment, set state PageCross
    if R::PAGE_CROSS && carry {
        state.op_state = OpState::PageCross;
    }
}

// Read byte and add index x or y to it
fn read_add_index<R: ReadType, I: Register>(state: &mut State) {
    /// Read Byte
    R::pre(state);
    state.read();
    R::post(state);

    state.bus.wire += I::get(state);
}

/// Increment wire by Y and store locally. do a read from somewhere, set new low to stored value and new high to read value. Optionally check for Page Crosses.
fn read_add_y<R: ReadType>(state: &mut State) {
    /// Increment previous read by Y.
    let (low, carry) = state.bus.wire.carrying_add(state.cpu.y, false);
    /// Read Byte
    R::pre(state);
    state.read();
    R::post(state);

    state.bus.low = low;
    state.bus.high = state.bus.wire;

    /// If high byte needs increment, set state PageCross
    if carry {
        state.op_state = OpState::PageCross;
    }
}

fn branch<M: MathOp>(state: &mut State) {
    M::exec(state); // Branch MathOp should store operand in cpu.latch and set OpState::Branching in op_state
    state.bus.set(state.cpu.pc); // Useless read
    state.read();

    if state.op_state.contains(OpState::Branching) {
        let (new_addr, overflow) = state.bus.low.overflowing_add_signed(state.cpu.latch as i8);
        // Set Page cross if overflow
        state.op_state.set(OpState::PageCross, overflow);
        // Update program counter
        state.cpu.pc = u16::from_le_bytes([new_addr, state.bus.high]);
    } else {
        state.cpu.pc += 1;
    }
}

/// Running read-only instructions
fn read_run<R: ReadType, M: MathOp>(state: &mut State) {
    R::pre(state);
    state.read();
    R::post(state);
    M::exec(state)
}
/// Running write-only instructions
fn write_run<M: MathOp>(state: &mut State) {
    M::exec(state);
    state.write()
}

/// Read for RW op
fn read_eff(state: &mut State) {
    state.read();
}
/// Run RW op
fn rw_run<M: MathOp>(state: &mut State) {
    state.write();
    M::exec(state)
}
/// Write result of RW op
fn write_eff(state: &mut State) {
    state.write()
}


/// Run Math Op directly
fn run<M: MathOp>(state: &mut State) {
    M::exec(state)
}

/* /// Loads byte from tmp address
fn load_addr(state: &mut State) {
    if let TmpState::Address(addr) = state.op_state.tmp {
        state.op_state.tmp = TmpState::Byte({ state.bus.set(addr); state.read(); state.bus.wire });
    } else { panic!("load_addr expects to have a tmp address to read") }
}

/// Loads byte from zero page
fn load_zero(state: &mut State) {
    if let TmpState::Byte(low) = state.op_state.tmp {
        let addr = u16::from_be_bytes([0x00, low]);
        state.op_state.tmp = TmpState::Byte({ state.bus.set(addr); state.read(); state.bus.wire });
        state.cpu.pc += 1;
    } else { panic!("load_zero expects to have stored lower byte to use as zero-page address") }
}
fn inc_low_x(state: &mut State) {
    if let TmpState::Byte(low) = state.op_state.tmp {
        // TODO: Add fake read here
        state.op_state.tmp = TmpState::Byte(low + state.cpu.x);
    } else { panic!("inc_low_x expects to have stored lower byte to increment by x") }
}
fn inc_low_y(state: &mut State) {
    if let TmpState::Byte(low) = state.op_state.tmp {
        // TODO: Add fake read here
        state.op_state.tmp = TmpState::Byte(low + state.cpu.y);
    } else { panic!("inc_low_x expects to have stored lower byte to increment by y") }
}

/// Absolute Addressing + X (2 bytes)
fn xabs(state: &mut State) -> u16 {
    let high = { state.bus.set(state.cpu.pc + 1); state.read(); state.bus.wire };
    let low = { state.bus.set(state.cpu.pc + 2); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([high, low]) + state.cpu.x as u16;
    state.cpu.pc += 3; // Increment to next instruction
    addr
}
/// Absolute Addressing + Y (2 bytes)
fn yabs(state: &mut State) -> u16 {
    let high = { state.bus.set(state.cpu.pc + 1); state.read(); state.bus.wire };
    let low = { state.bus.set(state.cpu.pc + 2); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([high, low]) + state.cpu.y as u16;
    state.cpu.pc += 3; // Increment to next instruction
    addr
}
/// Absolute Indirect Addressing (2 bytes)
fn abs_ind(state: &mut State) -> u16 {
    // Read indirect address from program memory
    let high = { state.bus.set(state.cpu.pc + 1); state.read(); state.bus.wire };
    let low = { state.bus.set(state.cpu.pc + 2); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([high, low]);
    state.cpu.pc += 3; // Increment to next instruction
    // Read actual address from working memory
    let high = { state.bus.set(addr); state.read(); state.bus.wire };
    let low = { state.bus.set(addr + 1); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([high, low]);
    addr
}
/// Absolute Addressing at ZeroPage (1 byte)
fn zero_abs(state: &mut State) -> u16 {
    let low = { state.bus.set(state.cpu.pc + 1); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([0, low]);
    state.cpu.pc += 2; // Increment to next instruction
    addr
}
/// Absolute Addressing at ZeroPage + X (1 byte)
fn zero_xabs(state: &mut State) -> u16 {
    let low = { state.bus.set(state.cpu.pc + 1); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([0, low]) + state.cpu.x as u16;
    state.cpu.pc += 2; // Increment to next instruction
    addr
}
/// Absolute Addressing at ZeroPage + Y (1 byte)
fn zero_yabs(state: &mut State) -> u16 {
    let low = { state.bus.set(state.cpu.pc + 1); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([0, low]) + state.cpu.y as u16;
    state.cpu.pc += 2; // Increment to next instruction
    addr
}
/// $eff = Read 2 bytes from $00(nn + X)
fn x_zero_ind(state: &mut State) -> u16 {
    // Read indirect address from program memory
    let zero_low = { state.bus.set(state.cpu.pc + 1); state.read(); state.bus.wire } + state.cpu.x;
    let addr = u16::from_be_bytes([0x00, zero_low]);
    state.cpu.pc += 2; // Increment to next instruction
    // Read actual address from working memory
    let low = { state.bus.set(addr); state.read(); state.bus.wire };
    let high = { state.bus.set(addr + 1); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([high, low]);
    addr
}
/// hhll = Read 2 bytes from $00nn, $eff = $hh(ll + Y).   
fn zero_ind_y(state: &mut State) -> u16 {
    // Read indirect zero-page address from program memory
    let zero_low = { state.bus.set(state.cpu.pc + 1); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([0x00, zero_low]);
    
    // Read actual address from working memory and manipulate it
    let (low, carry) = { state.bus.set(addr); state.read(); state.bus.wire }.carrying_add(state.cpu.y, false);
    let (high, _) = { state.bus.set(addr + 1); state.read(); state.bus.wire }.carrying_add(0, carry);
    
    let addr = u16::from_be_bytes([high, low]);
    state.cpu.pc += 2; // Increment to next instruction
    
    addr
}

fn rel(state: &mut State) -> u16 {
    let high = { state.bus.set(state.cpu.pc + 1); state.read(); state.bus.wire };
    let low = { state.bus.set(state.cpu.pc + 2); state.read(); state.bus.wire };
    let addr = u16::from_be_bytes([high, low]);
    state.cpu.pc + addr
} */


/* /// Load to A
struct LDA;
impl InstructionType for LDA {
    type Data = u8;
    fn execute(state: &mut State, data: Self::Data) {
        state.cpu.a = data;
    }
}
struct LDX;
impl InstructionType for LDX {
    type Data = u8;
    fn execute(state: &mut State, data: Self::Data) {
        state.cpu.x = data;
    }
}

struct LDY;
impl InstructionType for LDY {
    type Data = u8;
    fn execute(state: &mut State, data: Self::Data) {
        state.cpu.y = data;
    }
}

struct STA;
impl InstructionType for STA {
    type Data = u16;
    fn execute(state: &mut State, data: Self::Data) {
        state.mem.set(data, state.cpu.a)
    }
}
struct STX;
impl InstructionType for STA {
    type Data = u16;
    fn execute(state: &mut State, data: Self::Data) {
        state.mem.set(data, state.cpu.x)
    }
}
struct STY;
impl InstructionType for STA {
    type Data = u16;
    fn execute(state: &mut State, data: Self::Data) {
        state.mem.set(data, state.cpu.y)
    }
}

/// Transfer A to X
struct TAX;
/// Transfer X to A
struct TXA;
struct PHA;
struct PLA; */