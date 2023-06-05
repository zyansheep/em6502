//! Instructions are composed of "micro-operations" one of which runs each cycle that the instruction is active.
//! Each micro-op can in general do one memory operation and one ALU operation in parallel.
//! For instructions that don't modify memory, a memory operation is stil done because the min cycle count per instruction is 2
//! The micro-op cycle structure is outlined by this document: https://www.atarihq.com/danb/files/64doc.txt

mod table;
mod math;
mod reg;
pub use math::*;
pub use reg::*;
pub use table::INSTR_SET;
use std::{collections::HashMap, ops::Shl, io::Read, marker::PhantomData, cmp::Ordering, fmt::Write};

use crate::{State, CpuFlags, OpState, Logging};

const fn join<const A: usize, const B: usize>(a: InstrPipeline<A>, b: InstrPipeline<B>) -> InstrPipeline<{A + B}> {
    let mut out: [fn(&mut State); {A + B}] = [State::read; {A + B}];
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
/// Exec B, read, exec A.
fn read<B: MathOp, A: MathOp>(state: &mut State) {
    B::exec(state);
    state.read();
    A::exec(state);
}
fn check_pagecross(state: &mut State) {
    if state.op_state.contains(OpState::PageCross) {
        state.cpu.io.high = state.cpu.io.high.wrapping_add(1); // increment high
        state.op_state.remove(OpState::PageCross); // remove pagecross flag
    }
    state.read(); // read
}
/// Optimized read, allows skipping extra read if pagecross did not occur
/// If pagecross, update effective address high, disable pagecross flag, decrement cycle_idx (so that this runs again)
fn opt_read<B: MathOp, A: MathOp>(state: &mut State) {
    if state.op_state.contains(OpState::PageCross) {
        check_pagecross(state);
        // execute this instruction again
        state.cycle_idx = state.cycle_idx.wrapping_sub(1);
    } else {
        read::<B, A>(state);
    }
}
/// Exec B, write, exec A.
fn write<B: MathOp, A: MathOp>(state: &mut State) {
    B::exec(state);
    state.write();
    A::exec(state);
}

type InstrPipeline<const S: usize> = [fn(&mut State); S];
const fn implied<M: MathOp>() -> InstrPipeline<1> {
    [read::<SetAddrPC, M>] // read next instruction byte (and throw it away)
}
/// immediate addressing
const fn immediate<M: MathOp>() -> InstrPipeline<1> {
    [read::<SetAddrPC, Seq<ReadFirst, M>>]
}

// BRK instruction
const BRK: InstrPipeline<6> = [
    read::<NOP, IncPC>,             // read next instruction byte (and throw it away), increment PC
    write::<SetAddrStack, PUSH_STACK<PCH>>,             // push PCH onto stack, decrement SP
    write::<SetAddrStack, PUSH_STACK<PCL>>,             // push PCL onto stack, decrement SP
    write::<SetAddrStack, PUSH_STACK<FLAGS_WITH_BRK>>,  // push FLAGS on stack (with B flag set), decrement S
    read::<SetAddrConst<0xFE, 0xFF>, Fetch<PCL>>,          // fetch PCL from 0xFFFE
    read::<SetAddrConst<0xFF, 0xFF>, Fetch<PCH>>           // fetch PCH from 0xFFFF
];
/// Return from Interrupt
const RTI: InstrPipeline<5> = [
    read::<SetAddrPC, NOP>, // read next instruction byte (and throw it away)
    read::<SetAddrStack, INC<SP>>, // increment SP
    read::<SetAddrStack, Seq<Fetch<FLAGS_REMOVE_BREAK>, INC<SP>>>, // pull P from stack, increment SP
    read::<SetAddrStack, Seq<Fetch<PCL>, INC<SP>>>, // pull PCL from stack, increment SP
    read::<SetAddrStack, Fetch<PCH>>, // pull PCH from stack
];
/// Push register to stack (PHA, PHP)
const fn push_stack<R: Register>() -> InstrPipeline<2> {
    [
        read::<SetAddrPC, NOP>, // read next instruction byte (and throw it away)
        write::<PUSH_STACK<R>, NOP>,
    ]
}
/// Pull register from stack (PLA, PLP)
const fn pull_stack<R: Register, E: MathOp>() -> InstrPipeline<3> {
    [
        read::<SetAddrPC, NOP>, // read next instruction byte (and throw it away)
        read::<NOP, INC<SP>>,
        read::<SetAddrStack, Seq<Fetch<R>, E>>,
    ]
}

/// Jump Subroutine
const JSR: InstrPipeline<5> = [
    read::<SetAddrPC, ReadFirst>, // fetch low address byte, increment PC
    read::<SetAddrStack, NOP>, // internal operation (predecrement S?)
    write::<PUSH_STACK<PCH>, NOP>, // push PCH on stack, decrement S
    write::<PUSH_STACK<PCL>, NOP>, // push PCL on stack, decrement S
    read::<SetAddrPC, Seq<ReadSecond, SET_PC<FIRST, SECOND>>>, // copy low address byte to PCL, fetch high address byte to PCH
];
/// Return Subroutine
const RTS: InstrPipeline<5> = [
    read::<SetAddrPC, NOP>, // read next instruction byte (and throw it away)
    read::<SetAddrStack, INC<SP>>, // increment S
    read::<SetAddrStack, Seq<Fetch<PCL>, INC<SP>>>, // pull PCL from stack, increment SP
    read::<SetAddrStack, Fetch<PCH>>, // pull PCH from stack
    read::<NOP, IncPC>, // increment PC
];

/// absolute addressing
const fn absolute<const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{2 + A}> {
    join([
        read::<SetAddrPC, ReadFirst>, // fetch low address byte, increment PC
        read::<SetAddrPC, ReadSecondAddr> // fetch high address byte, increment PC
        ], op)
}
/// zero page addressing
const fn zeropage<const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{1 + A}> {
    join([read::<SetAddrPC, Seq<ReadFirst, SetAddrZero<FIRST>>>], op)
}
/// zero page indexed addressing
const fn zeropage_indexed<I: Register, const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{2 + A}> {
    join([
        read::<SetAddrPC, ReadFirst>, // fetch address, increment PC
        read::<SetAddrZero<FIRST>, AddIndex<I, false>> // read from address, add index register
        ], op)
}
/// absolute indexed addressing, may cause page cross
const fn absolute_indexed<I: Register, const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{2 + A}> {
    join([
        read::<SetAddrPC, ReadFirst>, // fetch low byte of address, increment PC
        read::<SetAddrPC, Seq<ReadSecondAddr, AddIndex<I, true>>>, // fetch high byte of address, add index register to low address byte, increment PC
        // read::<NOP, NOP> // read from effective address, fix the high byte of effective address (this is done during page fault)
        ], op)
}
/// relative addressing
const fn branch_if<const FLAG: CpuFlags, const STATE: bool>() -> InstrPipeline<1> {[do_branch::<FLAG, STATE>]}

// fetch opcode of next instruction, If branch is taken, add operand to PCL. Otherwise increment PC.
// may cause pagecross. If called with pagecross, increments PCH
fn do_branch<const FLAG: CpuFlags, const STATE: bool>(state: &mut State) {
    SetAddrPC::exec(state);

    // If in process of page cross, do early return.
    if state.op_state.contains(OpState::PageCross) {
        state.op_state.remove(OpState::PageCross);
        PCH::set(state, PCH::get(state).wrapping_add(1)); // increment PCH
        return
    }

    state.read();

    // log operand
    state.cpu.first = Some(state.cpu.io.wire);
    // Check if specific cpu FLAG equals required STATE
    if state.cpu.flags.contains(FLAG) == STATE {
        state.op_state.set(OpState::Branching, true);
        // inc pc to next instruction
        IncPC::exec(state);
        SetAddrPC::exec(state);
        /// If branching, add operand to MEM_LOW, setting page cross if needed
        AddIndex::<FIRST, true>::exec(state);
        // make sure PCL reflects MEM_LOW
        MV::<MEM_LOW, PCL>::exec(state);
        
        // if pagecross, run this command again
        if state.op_state.contains(OpState::PageCross) {
            state.cycle_idx = state.cycle_idx.wrapping_sub(1);
        }
    } else {
        IncPC::exec(state); // if no branch, increment PC
    }
}

/// indexed indirect addressing
const fn indexed_indirect<const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{4 + A}> {
    join([
        read::<SetAddrPC, ReadFirst>, // fetch pointer address, increment PC
        read::<SetAddrZero<FIRST>, AddIndex<X, false>>, // read from address, add X to address.
        read::<NOP, Fetch<LATCH>>, // fetch effective address low
        read::<INC<MEM_LOW>, SetAddr<LATCH, BUS>>, // fetch effective address high
        ], op)
}
/// indirect indexed addressing, may cause page cross
const fn indirect_indexed<const A: usize>(op: InstrPipeline<A>) -> InstrPipeline<{3 + A}> {
    join([
        read::<SetAddrPC, ReadFirst>, // fetch pointer address, increment PC
        read::<SetAddrZero<FIRST>, Fetch<LATCH>>, // fetch effective address low
        read::<INC<MEM_LOW>, Seq<SetAddr<LATCH, BUS>, AddIndex<Y, true>>>, // fetch effective address high, add Y to low byte of effective address
        ],op) // do read/write op, checking page cross if needed
}
// absolute indirect (JMP) instruction
const fn absolute_indirect_jmp() -> InstrPipeline<4> {
    [
        read::<SetAddrPC, ReadFirst>,                           // fetch pointer low, increment PC
        read::<SetAddrPC, ReadSecond>,                          // fetch pointer high, increment PC. 
        read::<SetAddrOP, Fetch<LATCH>>,                      // fetch low address to latch
        read::<INC<MEM_LOW>, Seq<Fetch<PCH>, MV<LATCH, PCL>>> // fetch PCH, copy latch to PCL.
    ]
}


const fn read_op<M: MathOp>() -> InstrPipeline<1> {
    [opt_read::<LogEff, M>]
}
const fn write_op<M: MathOp>() -> InstrPipeline<1> {
    [write::<M, LogEff>]
}
/// write_op, but always checks if page crossed
const fn write_op_pc<M: MathOp>() -> InstrPipeline<2> {
    [check_pagecross, write::<M, LogEff>]
}
const fn rw_op<M: MathOp>() -> InstrPipeline<3> {
    [State::read, write::<LogEff, M>, State::write]
}
const fn rw_op_pc<M: MathOp>() -> InstrPipeline<4> {
    [check_pagecross, State::read, write::<LogEff, M>, State::write]
}