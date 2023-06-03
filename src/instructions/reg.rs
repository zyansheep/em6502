use super::*;

/// Allows fetching and storing to and from registers.
pub trait Register {
    fn get(state: &State) -> u8;
    fn set(state: &mut State, val: u8);
}

/// Accumulator Register
pub struct ACC;
impl Register for ACC {
    fn get(state: &State) -> u8 { state.cpu.a }
    fn set(state: &mut State, val: u8) { state.cpu.a = val; }
}
/// X Register
pub struct X;
impl Register for X {
    fn get(state: &State) -> u8 { state.cpu.x }
    fn set(state: &mut State, val: u8) { state.cpu.x = val; }
}
/// Y Register
pub struct Y;
impl Register for Y {
    fn get(state: &State) -> u8 { state.cpu.y }
    fn set(state: &mut State, val: u8) { state.cpu.y = val; }
}
/// Stack Pointer
pub struct SP;
impl Register for SP {
    fn get(state: &State) -> u8 { state.cpu.sp }
    fn set(state: &mut State, val: u8) { state.cpu.sp = val; }
}
/// Program Counter Low
pub struct PCL;
impl Register for PCL {
    fn get(state: &State) -> u8 { state.cpu.pc_get()[0] }
    fn set(state: &mut State, val: u8) {
        let mut pc = state.cpu.pc_get();
        pc[0] = val;
        state.cpu.pc_set(pc);
    }
}
/// Program Counter High
pub struct PCH;
impl Register for PCH {
    fn get(state: &State) -> u8 { state.cpu.pc_get()[1] }
    fn set(state: &mut State, val: u8) {
        let mut pc = state.cpu.pc_get();
        pc[1] = val;
        state.cpu.pc_set(pc);
    }
}
/// Latch Register
pub struct LATCH;
impl Register for LATCH {
    fn get(state: &State) -> u8 { state.cpu.latch }
    fn set(state: &mut State, val: u8) { state.cpu.latch = val; }
}
/// First Operand Register
pub struct FIRST;
impl Register for FIRST {
    fn get(state: &State) -> u8 { state.cpu.first.unwrap_or(0) }
    fn set(state: &mut State, val: u8) { state.cpu.first = Some(val); }
}
/// Second Operand Register
pub struct SECOND;
impl Register for SECOND {
    fn get(state: &State) -> u8 { state.cpu.second.unwrap_or(0) }
    fn set(state: &mut State, val: u8) { state.cpu.second = Some(val); }
}


pub struct FLAGS_REMOVE_BREAK;
impl Register for FLAGS_REMOVE_BREAK {
    fn get(state: &State) -> u8 {
        state.cpu.flags.bits()
    }
    fn set(state: &mut State, val: u8) {
        state.cpu.flags = CpuFlags::from_bits_retain(val);
        state.cpu.flags.remove(CpuFlags::Break);
    }
}
/// CPU Flags register
pub struct FLAGS;
impl Register for FLAGS {
    fn get(state: &State) -> u8 {
        state.cpu.flags.bits()
    }
    fn set(state: &mut State, val: u8) {
        state.cpu.flags = CpuFlags::from_bits_retain(val);
    }
}

/// P Register, but only implements GET with B flags set.
pub struct FLAGS_WITH_BRK;
impl Register for FLAGS_WITH_BRK {
    fn get(state: &State) -> u8 { state.cpu.flags.union(CpuFlags::Break).bits() }
    fn set(state: &mut State, val: u8) { unimplemented!() }
}

pub struct BUS;
impl Register for BUS {
    fn get(state: &State) -> u8 { state.cpu.io.wire }
    fn set(state: &mut State, val: u8) { state.cpu.io.wire = val; }
}
pub struct MEM_LOW;
impl Register for MEM_LOW {
    fn get(state: &State) -> u8 { state.cpu.io.low }
    fn set(state: &mut State, val: u8) { state.cpu.io.low = val; }
}
pub struct MEM_HIGH;
impl Register for MEM_HIGH {
    fn get(state: &State) -> u8 { state.cpu.io.high }
    fn set(state: &mut State, val: u8) { state.cpu.io.high = val; }
}
pub struct ConstReg<const VAL: u8>;
impl<const VAL: u8> Register for ConstReg<VAL> {
    fn get(state: &State) -> u8 {
        VAL
    }
    fn set(state: &mut State, val: u8) {
        unimplemented!()
    }
}