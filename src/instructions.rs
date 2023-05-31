//! Instructions are composed of "micro-operations" one of which runs each cycle that the instruction is active.
//! Each micro-op can in general do one memory operation and one ALU operation in parallel.
//! For instructions that don't modify memory, a memory operation is stil done because the min cycle count per instruction is 2
//! The micro-op cycle structure is outlined by this document: https://www.atarihq.com/danb/files/64doc.txt

mod table;
pub use table::INSTR_SET;
use std::{collections::HashMap, ops::Shl, io::Read, marker::PhantomData, cmp::Ordering};

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
        state.cpu.flags.set(CpuFlags::Negative, (a_new & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, a_new == 0);
        // Overflow is set if addition changed sign bit.
        state.cpu.flags.set(CpuFlags::Overflow, (state.cpu.a & 0b1000_0000) != (a_new & 0b1000_0000));
        state.cpu.a = a_new;
    }
}

/// Compare A with contents of memory. Carry is set if memory <= A. Z is set if they are equal. Negative is set if A < memory.
struct CMP<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for CMP<I> {
    fn exec(state: &mut State) {
        // Unsigned addition overflow changes the carry flag
        let ordering = I::get(state).cmp(&state.bus.wire);
        state.cpu.flags.set(CpuFlags::Carry, ordering.is_ge());
        state.cpu.flags.set(CpuFlags::Negative, ordering.is_lt());
        state.cpu.flags.set(CpuFlags::Zero, ordering.is_eq());
    }
}

/// Subtract Memory from Accumulator with Borrow.
struct SBC;
impl MathOp for SBC {
    fn exec(state: &mut State) {
        // Carry is the reverse ("complement") of carry flag.
        let (a_new, borrow) = state.cpu.a.borrowing_sub(state.bus.wire, !state.cpu.flags.contains(CpuFlags::Carry));
        state.cpu.flags.set(CpuFlags::Carry, !borrow);
        // Set overflow bit if most significant bit changed
        state.cpu.flags.set(CpuFlags::Negative, (a_new & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Overflow, (state.cpu.a & 0b1000_0000) != (a_new & 0b1000_0000));
        state.cpu.flags.set(CpuFlags::Zero, a_new == 0);
        state.cpu.a = a_new;
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
    }
}

/// Performs OR with accumulator
struct ORA;
impl MathOp for ORA {
    fn exec(state: &mut State) {
        // Perform assign AND
        state.cpu.a |= state.bus.wire;
        state.cpu.flags.set(CpuFlags::Negative, (state.cpu.a & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, state.cpu.a == 0);
    }
}

/// Performs XOR with accumulator
struct EOR;
impl MathOp for EOR {
    fn exec(state: &mut State) {
        // Perform assign AND
        state.cpu.a ^= state.bus.wire;
        state.cpu.flags.set(CpuFlags::Negative, (state.cpu.a & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, state.cpu.a == 0);
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
    }
}

/// Shift Left a Register
struct ASL<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for ASL<I> {
    fn exec(state: &mut State) {
        let reg = I::get(state);
        // Carry bit is bit that is shifted out
        let carry = (reg & 0b1000_0000) != 0;
        // Do shift
        I::set(state, reg << 1);
        // Flags set accordingly
        state.cpu.flags.set(CpuFlags::Carry, carry);
        state.cpu.flags.set(CpuFlags::Negative, state.bus.wire & 0b1000_0000 != 0);
        state.cpu.flags.set(CpuFlags::Zero, reg == 0);
    }
}
/// Rotate Left a Register (ASL but with input carry)
struct ROL<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for ROL<I> {
    fn exec(state: &mut State) {
        let reg = I::get(state);
        // Carry bit is bit that is shifted out
        let carry = (reg & 0b1000_0000) != 0;
        // Input carry is bit that is shifted in
        let input_carry = if state.cpu.flags.contains(CpuFlags::Carry) { 0x01u8 } else { 0x00u8 };
        // Do shift (then add input carry)
        let reg = (reg << 1) | input_carry;
        
        I::set(state, reg);

        // Set bits accordingly
        state.cpu.flags.set(CpuFlags::Carry, carry);
        state.cpu.flags.set(CpuFlags::Negative, reg & 0b1000_0000 != 0);
        state.cpu.flags.set(CpuFlags::Zero, reg == 0);
    }
}

/// Shift Right a Register
struct LSR<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for LSR<I> {
    fn exec(state: &mut State) {
        let reg = I::get(state);
        // Carry bit is bit that is shift out
        let carry = (reg & 0b0000_0001) != 0;
        let reg = reg >> 1;
        I::set(state, reg);
        state.cpu.flags.set(CpuFlags::Carry, carry);
        state.cpu.flags.set(CpuFlags::Negative, false); // Negative bit is shifted in, so it is always false
        state.cpu.flags.set(CpuFlags::Zero, reg == 0);
    }
}
/// Rotate Right a Register (Shift right but with input carry)
struct ROR<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for ROR<I> {
    fn exec(state: &mut State) {
        let reg = I::get(state);
        // Carry bit is the one shifted out
        let carry = (reg & 0b0000_0001) != 0;
        let input_carry = state.cpu.flags.contains(CpuFlags::Carry);

        // Do shift then add input carry
        let reg = reg >> 1 | if input_carry { 0b1000_0000u8 } else { 0x0u8 };
        I::set(state, reg);
        
        state.cpu.flags.set(CpuFlags::Carry, carry);
        state.cpu.flags.set(CpuFlags::Negative, input_carry); // MSB is one shifted in
        state.cpu.flags.set(CpuFlags::Zero, reg == 0);
    }
}

struct Branch<const FLAG: CpuFlags, const STATE: bool>;
impl<const FLAG: CpuFlags, const STATE: bool> MathOp for Branch<FLAG, STATE> {
    fn exec(state: &mut State) {
        /// Check if specific cpu FLAG equals required STATE
        if state.cpu.flags.contains(FLAG) == STATE {
            state.op_state.set(OpState::Branching, true);
            state.cpu.latch = state.bus.wire;
        }
    }
}
struct CL<const FLAG: CpuFlags>;
impl<const FLAG: CpuFlags> MathOp for CL<FLAG> {
    fn exec(state: &mut State) {
        state.cpu.flags.remove(FLAG);
    }
}
struct SET<const FLAG: CpuFlags>;
impl<const FLAG: CpuFlags> MathOp for SET<FLAG> {
    fn exec(state: &mut State) {
        state.cpu.flags.remove(FLAG);
    }
}


/// Uses bus wire as low byte, and reads next byte in mem as high. sets PC counter accordingly.
/// Can be used for both absolute and indirect jumps
struct JMP;
impl MathOp for JMP {
    fn exec(state: &mut State) {
        state.cpu.pc_set([state.bus.low, state.bus.high]);
    }
}

struct NOP;
impl MathOp for NOP {
    fn exec(state: &mut State) {}
}

struct SEI;
impl MathOp for SEI {
    fn exec(state: &mut State) {
        state.cpu.flags.set(CpuFlags::InterruptDisable, true);
    }
}

struct LD<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for LD<I> {
    fn exec(state: &mut State) {
        I::set(state, state.bus.wire);
    }
}
struct ST<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for ST<I> {
    fn exec(state: &mut State) {
        state.bus.wire = I::get(state);
    }
}
struct TR<I1: Register, I2: Register>(PhantomData<I1>, PhantomData<I2>);
impl<I1: Register, I2: Register> MathOp for TR<I1, I2> {
    fn exec(state: &mut State) {
        I2::set(state, I1::get(state));
    }
}

struct INC<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for INC<I> {
    fn exec(state: &mut State) {
        I::set(state, state.bus.wire.wrapping_add(1));
    }
}

struct DEC<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for DEC<I> {
    fn exec(state: &mut State) {
        I::set(state, I::get(state).wrapping_sub(1));
    }
}

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
    [read_byte::<PCRead>, read_addr::<PCRead>, read_to_reg::<IncRead, LATCH>, read_high_reg_low::<LATCH, RegRead, JMP>]
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
    fn post(state: &mut State) { state.bus.low += 1; }
    const PAGE_CROSS: bool = false;
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
}

/// Sets bus high byte to zero.
struct ZeroRead;
impl ReadType for ZeroRead {
    fn pre(state: &mut State) {
        state.bus.low = state.bus.wire;
        state.bus.high = 0;
    }
    const PAGE_CROSS: bool = false;
}
struct ConstRead<const A: u16>;
impl<const A: u16> ReadType for ConstRead<A> {
    fn pre(state: &mut State) { state.bus.set(A) }
    const PAGE_CROSS: bool = false;
}

trait Register {
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
struct BREAK_FLAGS;
impl Register for BREAK_FLAGS {
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
struct BUS;
impl Register for BUS {
    fn get(state: &State) -> u8 { state.bus.wire }
    fn set(state: &mut State, val: u8) { state.bus.wire = val; }
}

/// Push register onto stack
fn push_stack<I: Register>(state: &mut State) {
    state.bus.wire = I::get(state);
    state.bus.high = 0x10;
    state.bus.low = state.cpu.sp;
    state.write();
    state.cpu.sp = state.cpu.sp.wrapping_sub(1); // decrement stack pointer after writing
}
/// Pop from stack to register
fn pop_stack<I: Register>(state: &mut State) {
    state.bus.high = 0x10;
    state.bus.low = state.cpu.sp;
    state.read();
    I::set(state, state.bus.wire);
    state.cpu.sp = state.cpu.sp.wrapping_add(1); // increment stack pointer after popping
}

/// Reads current byte to register and increments address
fn read_to_reg<R: ReadType, I: Register>(state: &mut State) {
    R::pre(state);
    state.read();
    R::post(state);
    I::set(state, state.bus.wire);
}

/// sets higher byte from next memory location and sets lower byte from register. Runs MathOp
fn read_high_reg_low<I: Register, R: ReadType, M: MathOp>(state: &mut State) {
    // Read high from mem
    R::pre(state);
    state.read();
    R::post(state);
    state.bus.high = state.bus.wire;

    // Read low from reg
    state.bus.low = I::get(state);
    
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
    let low = state.bus.wire;
    
    state.bus.low += 1;
    R::pre(state);
    state.read();
    R::post(state);

    state.bus.high = state.bus.wire;
    state.bus.low = low;
}

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
/// Do branch to relative address if M sets OpState::Branching.
fn branch<M: MathOp>(state: &mut State) {
    M::exec(state); // Branch MathOp should store operand in cpu.latch and set OpState::Branching in op_state
    state.bus.set(state.cpu.pc); // Useless read because the spec says so
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