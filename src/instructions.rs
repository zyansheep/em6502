//! Instructions are composed of "micro-operations" one of which runs each cycle that the instruction is active.
//! Each micro-op can in general do one memory operation and one ALU operation in parallel.
//! For instructions that don't modify memory, a memory operation is stil done because the min cycle count per instruction is 2
//! The micro-op cycle structure is outlined by this document: https://www.atarihq.com/danb/files/64doc.txt

mod table;
mod math;
pub use math::*;
pub use table::INSTR_SET;
use std::{collections::HashMap, ops::Shl, io::Read, marker::PhantomData, cmp::Ordering};

use crate::{State, CpuFlags, OpState, Logging};

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
const fn absolute_indirect<M: MathOp>() -> InstrPipeline<4> {
    [read_byte::<PCRead>, read_addr::<PCRead>, read_to_reg::<IncRead, LATCH>, read_high_reg_low::<RegRead, LATCH, JMP>]
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
    join([read_byte::<PCRead>, read_add_index::<ZeroRead, XIndex>, read_byte::<IncRead>, read_addr::<RegRead>], op)
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
    fn pre(state: &mut State) {}
    fn post(state: &mut State) {}
    const PAGE_CROSS: bool = true;
}

/// Do nothing
struct RegRead;
impl ReadType for RegRead {}
/// Read the next memory address, not handling page crossing
struct IncRead;
impl ReadType for IncRead {
    fn post(state: &mut State) { state.cpu.io.low += 1; }
    const PAGE_CROSS: bool = false;
}

/// Increment read from program counter and increment pc after reading.
struct PCRead;
impl ReadType for PCRead {
    fn pre(state: &mut State) {
        state.cpu.io.set(state.cpu.pc);
    }
    fn post(state: &mut State) {
        state.cpu.pc += 1;
    }
}

/// Sets bus high byte to zero.
struct ZeroRead;
impl ReadType for ZeroRead {
    fn pre(state: &mut State) {
        state.cpu.io.low = state.cpu.io.wire;
        state.cpu.io.high = 0;
    }
    const PAGE_CROSS: bool = false;
}
struct ConstRead<const A: u16>;
impl<const A: u16> ReadType for ConstRead<A> {
    fn pre(state: &mut State) { state.cpu.io.set(A) }
    const PAGE_CROSS: bool = false;
}

pub trait Register {
    fn get(state: &State) -> u8;
    fn set(state: &mut State, val: u8);
}
struct Acc;
impl Register for Acc {
    fn get(state: &State) -> u8 { state.cpu.a }
    fn set(state: &mut State, val: u8) { state.cpu.a = val; }
}
struct XIndex;
impl Register for XIndex {
    fn get(state: &State) -> u8 { state.cpu.x }
    fn set(state: &mut State, val: u8) { state.cpu.x = val; }
}
struct YIndex;
impl Register for YIndex {
    fn get(state: &State) -> u8 { state.cpu.y }
    fn set(state: &mut State, val: u8) { state.cpu.y = val; }
}
struct PCL;
impl Register for PCL {
    fn get(state: &State) -> u8 { state.cpu.pc_get()[0] }
    fn set(state: &mut State, val: u8) {
        let mut pc = state.cpu.pc_get();
        pc[0] = val;
        state.cpu.pc_set(pc);
    }
}
struct PCH;
impl Register for PCH {
    fn get(state: &State) -> u8 { state.cpu.pc_get()[1] }
    fn set(state: &mut State, val: u8) {
        let mut pc = state.cpu.pc_get();
        pc[1] = val;
        state.cpu.pc_set(pc);
    }
}
struct LATCH;
impl Register for LATCH {
    fn get(state: &State) -> u8 { state.cpu.latch }
    fn set(state: &mut State, val: u8) { state.cpu.latch = val; }
}
struct FLAGS;
impl Register for FLAGS {
    fn get(state: &State) -> u8 {
        state.cpu.flags.bits()
    }
    fn set(state: &mut State, val: u8) {
        state.cpu.flags = CpuFlags::from_bits_retain(val);
    }
}
/// CPU Flags register, but with Break value set
struct FLAGS_WITH_BREAK;
impl Register for FLAGS_WITH_BREAK {
    fn get(state: &State) -> u8 {
        state.cpu.flags.union(CpuFlags::Break).bits()
    }
    fn set(state: &mut State, val: u8) {
        state.cpu.flags = CpuFlags::from_bits_retain(val);
    }
}
struct STACK_POINTER;
impl Register for STACK_POINTER {
    fn get(state: &State) -> u8 { state.cpu.sp }
    fn set(state: &mut State, val: u8) { state.cpu.sp = val; }
}
pub struct BUS;
impl Register for BUS {
    fn get(state: &State) -> u8 { state.cpu.io.wire }
    fn set(state: &mut State, val: u8) { state.cpu.io.wire = val; }
}

/// Push register onto stack
fn push_stack<I: Register>(state: &mut State) {
    state.cpu.io.wire = I::get(state);
    state.cpu.io.high = 0x10;
    state.cpu.io.low = state.cpu.sp;
    state.write();
    state.cpu.sp = state.cpu.sp.wrapping_sub(1); // decrement stack pointer after writing
}
/// Pop from stack to register
fn pop_stack<I: Register>(state: &mut State) {
    state.cpu.io.high = 0x10;
    state.cpu.io.low = state.cpu.sp;
    state.read();
    I::set(state, state.cpu.io.wire);
    state.cpu.sp = state.cpu.sp.wrapping_add(1); // increment stack pointer after popping
}

/// Reads current byte to register and increments address
fn read_to_reg<R: ReadType, I: Register>(state: &mut State) {
    R::pre(state);
    state.read();
    R::post(state);
    I::set(state, state.cpu.io.wire);
}

/// sets higher byte from next memory location and sets lower byte from register. Runs MathOp
fn read_high_reg_low<R: ReadType, I: Register, M: MathOp>(state: &mut State) {
    // Read high from mem
    R::pre(state);
    state.read();
    R::post(state);
    state.cpu.io.high = state.cpu.io.wire;

    // Read low from reg
    state.cpu.io.low = I::get(state);
    
    // Run OP
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
    let low = state.cpu.io.wire;
    
    R::pre(state);
    state.read();
    R::post(state);

    state.cpu.io.high = state.cpu.io.wire;
    state.cpu.io.low = low;
}

/// Increment previous read by index x or y, use as low byte. read new byte for high byte. Handle Page crossing
fn add_index_low_read_high<R: ReadType, I: Register>(state: &mut State) {
    /// Increment previous read by index.
    let (low, carry) = state.cpu.io.wire.carrying_add(I::get(state), false);
    /// Read Byte
    R::pre(state);
    state.read();
    R::post(state);

    state.cpu.io.low = low;
    state.cpu.io.high = state.cpu.io.wire;

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

    state.cpu.io.wire += I::get(state);
}

/// Increment wire by Y and store locally. do a read from somewhere, set new low to stored value and new high to read value. Optionally check for Page Crosses.
fn read_add_y<R: ReadType>(state: &mut State) {
    /// Increment previous read by Y.
    let (low, carry) = state.cpu.io.wire.carrying_add(state.cpu.y, false);
    /// Read Byte
    R::pre(state);
    state.read();
    R::post(state);

    state.cpu.io.low = low;
    state.cpu.io.high = state.cpu.io.wire;

    /// If high byte needs increment, set state PageCross
    if carry {
        state.op_state = OpState::PageCross;
    }
}

/// Do branch to relative address if M sets OpState::Branching.
fn branch<M: MathOp>(state: &mut State) {
    M::exec(state); // Branch MathOp should store operand in cpu.latch and set OpState::Branching in op_state
    state.cpu.io.set(state.cpu.pc); // Useless read because the spec says so
    state.read();
    
    state.log.operand = Some(state.cpu.latch);

    if state.op_state.contains(OpState::Branching) {
        let (new_addr, overflow) = state.cpu.io.low.overflowing_add(state.cpu.latch);
        // Set Page cross if overflow
        state.op_state.set(OpState::PageCross, overflow);
        // Update program counter
        state.cpu.pc = u16::from_le_bytes([new_addr, state.cpu.io.high]);
    }
}
/// Running read-only instructions
fn read_run<R: ReadType, M: MathOp>(state: &mut State) {
    R::pre(state);
    state.read();
    R::post(state);

    state.log.operand = Some(state.cpu.io.wire);

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
    state.log.operand = Some(state.cpu.io.wire);
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