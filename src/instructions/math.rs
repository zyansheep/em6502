use super::*;
pub trait MathOp {
    fn exec(state: &mut State);
}

/// Performs ADD with Carry between A and Memory Bus.
pub struct ADC;
impl MathOp for ADC {
    fn exec(state: &mut State) {
        // Unsigned addition overflow changes the carry flag
        let (a_new, carry) = state.cpu.a.carrying_add(state.cpu.io.wire, state.cpu.flags.contains(CpuFlags::Carry));
        state.cpu.flags.set(CpuFlags::Carry, carry);
        state.cpu.flags.set(CpuFlags::Negative, (a_new & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, a_new == 0);
        // Overflow is set if addition changed sign bit.
        state.cpu.flags.set(CpuFlags::Overflow, (state.cpu.a & 0b1000_0000) != (a_new & 0b1000_0000));
        state.cpu.a = a_new;
    }
}

/// Compare A with contents of memory. Carry is set if memory <= A. Z is set if they are equal. Negative is set if A < memory.
pub struct CMP<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for CMP<I> {
    fn exec(state: &mut State) {
        // Unsigned addition overflow changes the carry flag
        let ordering = I::get(state).cmp(&state.cpu.io.wire);
        state.cpu.flags.set(CpuFlags::Carry, ordering.is_ge());
        state.cpu.flags.set(CpuFlags::Negative, ordering.is_lt());
        state.cpu.flags.set(CpuFlags::Zero, ordering.is_eq());
    }
}

/// Subtract Memory from Accumulator with Borrow.
pub struct SBC;
impl MathOp for SBC {
    fn exec(state: &mut State) {
        // Carry is the reverse ("complement") of carry flag.
        let (a_new, borrow) = state.cpu.a.borrowing_sub(state.cpu.io.wire, !state.cpu.flags.contains(CpuFlags::Carry));
        state.cpu.flags.set(CpuFlags::Carry, !borrow);
        // Set overflow bit if most significant bit changed
        state.cpu.flags.set(CpuFlags::Negative, (a_new & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Overflow, (state.cpu.a & 0b1000_0000) != (a_new & 0b1000_0000));
        state.cpu.flags.set(CpuFlags::Zero, a_new == 0);
        state.cpu.a = a_new;
    }
}

/// Performs AND with A and Memory Bus and stores result in A.
pub struct AND;
impl MathOp for AND {
    fn exec(state: &mut State) {
        // Perform assign AND
        state.cpu.a &= state.cpu.io.wire;
        state.cpu.flags.set(CpuFlags::Negative, (state.cpu.a & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, state.cpu.a == 0);
    }
}

/// Performs OR with accumulator
pub struct ORA;
impl MathOp for ORA {
    fn exec(state: &mut State) {
        // Perform assign AND
        state.cpu.a |= state.cpu.io.wire;
        state.cpu.flags.set(CpuFlags::Negative, (state.cpu.a & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, state.cpu.a == 0);
    }
}

/// Performs XOR with accumulator
pub struct EOR;
impl MathOp for EOR {
    fn exec(state: &mut State) {
        // Perform assign AND
        state.cpu.a ^= state.cpu.io.wire;
        state.cpu.flags.set(CpuFlags::Negative, (state.cpu.a & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, state.cpu.a == 0);
    }
}


/// Performs AND with A and Memory Bus and sets flags.
pub struct BIT;
impl MathOp for BIT {
    fn exec(state: &mut State) {
        // Perform tmp AND.
        let res = state.cpu.a & state.cpu.io.wire;
        state.cpu.flags.set(CpuFlags::Overflow, (state.cpu.io.wire & 0b0100_0000) != 0);
        state.cpu.flags.set(CpuFlags::Negative, (res & 0b1000_0000) != 0);
    }
}

/// Shift Left a Register
pub struct ASL<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for ASL<I> {
    fn exec(state: &mut State) {
        let reg = I::get(state);
        // Carry bit is bit that is shifted out
        let carry = (reg & 0b1000_0000) != 0;
        // Do shift
        I::set(state, reg << 1);
        // Flags set accordingly
        state.cpu.flags.set(CpuFlags::Carry, carry);
        state.cpu.flags.set(CpuFlags::Negative, state.cpu.io.wire & 0b1000_0000 != 0);
        state.cpu.flags.set(CpuFlags::Zero, reg == 0);
    }
}
/// Rotate Left a Register (ASL but with input carry)
pub struct ROL<I: Register>(PhantomData<I>);
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
pub struct LSR<I: Register>(PhantomData<I>);
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
pub struct ROR<I: Register>(PhantomData<I>);
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

pub struct Branch<const FLAG: CpuFlags, const STATE: bool>;
impl<const FLAG: CpuFlags, const STATE: bool> MathOp for Branch<FLAG, STATE> {
    fn exec(state: &mut State) {
        /// Check if specific cpu FLAG equals required STATE
        if state.cpu.flags.contains(FLAG) == STATE {
            state.op_state.set(OpState::Branching, true);
            state.cpu.latch = state.cpu.io.wire;
        }
    }
}
pub struct CL<const FLAG: CpuFlags>;
impl<const FLAG: CpuFlags> MathOp for CL<FLAG> {
    fn exec(state: &mut State) {
        state.cpu.flags.remove(FLAG);
    }
}
pub struct SET<const FLAG: CpuFlags>;
impl<const FLAG: CpuFlags> MathOp for SET<FLAG> {
    fn exec(state: &mut State) {
        state.cpu.flags.remove(FLAG);
    }
}


/// Uses bus wire as low byte, and reads next byte in mem as high. sets PC counter accordingly.
/// Can be used for both absolute and indirect jumps
pub struct JMP;
impl MathOp for JMP {
    fn exec(state: &mut State) {
        state.cpu.pc_set([state.cpu.io.low, state.cpu.io.high]);
    }
}

pub struct NOP;
impl MathOp for NOP {
    fn exec(state: &mut State) {}
}

pub type ST<I: Register> = TR<I, BUS>;
pub type LD<I: Register> = TR<BUS, I>;
/// Transfer byte from one register to another
pub struct TR<I1: Register, I2: Register>(PhantomData<I1>, PhantomData<I2>);
impl<I1: Register, I2: Register> MathOp for TR<I1, I2> {
    fn exec(state: &mut State) {
        I2::set(state, I1::get(state));
    }
}

/// Increase register by one
pub struct INC<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for INC<I> {
    fn exec(state: &mut State) {
        I::set(state, state.cpu.io.wire.wrapping_add(1));
    }
}

/// Decrease register by one
pub struct DEC<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for DEC<I> {
    fn exec(state: &mut State) {
        I::set(state, I::get(state).wrapping_sub(1));
    }
}

pub struct BRK;
impl MathOp for BRK {
    fn exec(state: &mut State) {
        state.cpu.flags.set(CpuFlags::InterruptDisable, true);
    }
}