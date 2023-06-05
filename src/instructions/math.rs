use super::*;
pub trait MathOp {
    fn exec(state: &mut State);
}

/// Uses bus wire as low byte, and reads next byte in mem as high. sets PC counter accordingly.
/// Can be used for both absolute and indirect jumps
pub type JMP = SET_PC<MEM_LOW, MEM_HIGH>;

/// Do Nothing
pub struct NOP;
impl MathOp for NOP {
    fn exec(state: &mut State) {}
}

/// Transfer byte from one register to another
pub struct MV<R1: Register, R2: Register>(PhantomData<R1>, PhantomData<R2>);
impl<R1: Register, R2: Register> MathOp for MV<R1, R2> {
    fn exec(state: &mut State) {
        R2::set(state, R1::get(state));
    }
}

// Set Zero and Negative flags based on a given register
pub struct SetDefaultFlags<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for SetDefaultFlags<I> {
    fn exec(state: &mut State) {
        let reg = I::get(state);
        state.cpu.flags.set(CpuFlags::Zero, reg == 0);
        state.cpu.flags.set(CpuFlags::Negative, reg & 0b1000_0000 != 0);
    }
}

/// Join two MathOps together in a sequence
pub struct Seq<M1: MathOp, M2: MathOp>(PhantomData<M1>, PhantomData<M2>);
impl<M1: MathOp, M2: MathOp> MathOp for Seq<M1, M2> {
    fn exec(state: &mut State) {
        M1::exec(state);
        M2::exec(state);
    }
}

/// Store register to Memory
pub type Store<I> = MV<I, BUS>;
/// Fetch from memory to register
pub type Fetch<I> = MV<BUS, I>;
/// Like MV, but sets flags based on the resulting operand.
pub type MVF<R1, R2> = Seq<MV<R1, R2>, SetDefaultFlags<R2>>;
/// Load register from Memory, setting flags accordingly
pub type LDF<I> = MVF<BUS, I>;

/// Read first Operand, increment program counter
pub type ReadFirst = Seq<MV<BUS, FIRST>, IncPC>;
/// Read second Operand, increment program counter
pub type ReadSecond = Seq<MV<BUS, SECOND>, IncPC>;
/// Read second operand as high value of address
pub type ReadSecondAddr = Seq<ReadSecond, SetAddrOP>;

/// Log current address as effective address
pub struct LogEff;
impl MathOp for LogEff {
    fn exec(state: &mut State) {
        let addr = u16::from_le_bytes([state.cpu.io.low, state.cpu.io.high]);
        state.cpu.eff_addr = Some(addr);
    }
}
/// Sets address using two registers.
pub type SetAddr<L, H> = Seq<MV<L, MEM_LOW>, MV<H, MEM_HIGH>>;

/// Sets address to Program Counter
pub struct SetAddrPC;
impl MathOp for SetAddrPC {
    fn exec(state: &mut State) { state.cpu.io.set(state.cpu.pc) }
}
/// Sets address to Stack Pointer
pub type SetAddrStack = SetAddr<SP, ConstReg<01>>;
/// Sets address to first two operands
pub type SetAddrOP = SetAddr<FIRST, SECOND>;
/// Sets address to constant
pub type SetAddrConst<const L: u8, const H: u8> = SetAddr<ConstReg<L>, ConstReg<H>>;
/// Sets address to address in zeropage
pub type SetAddrZero<R> = SetAddr<R, ConstReg<0x00>>;

/// Increment Program Counter
pub struct IncPC;
impl MathOp for IncPC {
    fn exec(state: &mut State) {
        state.cpu.pc = state.cpu.pc.wrapping_add(1);
    }
}
// Sets Addr to Stack Pointer, Writes R to BUS, decrements Stack Pointer 
pub type PUSH_STACK<R> = Seq<SetAddrStack, Seq<Store<R>, DEC<SP>>>;


/// Add index register R to low address byte, optionally check for page crossing
pub struct AddIndex<R: Register, const CHECK_PAGE: bool>(PhantomData<R>);
impl<R: Register, const CHECK_PAGE: bool> MathOp for AddIndex<R, CHECK_PAGE> {
    fn exec(state: &mut State) {
        let (new_low, overflow) = state.cpu.io.low.overflowing_add(R::get(state));
        state.cpu.io.low = new_low;
        if CHECK_PAGE && overflow {
            state.op_state.set(OpState::PageCross, true);
        }
    }
}


/// A = A + Memory + Carry
/// Carry set of there is a carry
/// Negative set if sign bit set
/// Zero set if output is 0
/// Overflow set if sign bit changed
pub struct ADC;
impl MathOp for ADC {
    fn exec(state: &mut State) {
        // Unsigned addition overflow changes the carry flag
        let (a_new, carry) = state.cpu.a.carrying_add(state.cpu.io.wire, state.cpu.flags.contains(CpuFlags::Carry));
        state.cpu.flags.set(CpuFlags::Carry, carry);
        // Calculate overflow using sign bits.
        let a_sign = state.cpu.a & 0b1000_0000 != 0;
        let bus_sign = state.cpu.io.wire & 0b1000_0000 != 0;
        let res_sign = (a_new & 0b1000_0000) != 0;
        let overflow = (a_sign ^ res_sign) & (bus_sign ^ res_sign);
        state.cpu.flags.set(CpuFlags::Overflow, overflow);

        state.cpu.a = a_new;
        // Set other flags
        SetDefaultFlags::<ACC>::exec(state);
    }
}

/// Compare Register with Memory.
/// Carry set if  A >= Memory.
/// Negative set if sign bit of A - Memory is set.
/// Zero set if A == Memory
pub struct CMP<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for CMP<I> {
    fn exec(state: &mut State) {
        // Unsigned addition overflow changes the carry flag
        let reg = I::get(state);
        let mem = state.cpu.io.wire;
        let res = reg.wrapping_sub(mem);
        // println!("{:02X?} - {:02X?} = {:02X?} + {borrow}", reg, mem, res);
        state.cpu.flags.set(CpuFlags::Negative, (res & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, reg == mem);
        state.cpu.flags.set(CpuFlags::Carry, reg >= mem);
    }
}
#[test]
fn test_cmp() {
    let state = &mut State::new();
    state.cpu.a = 10;
    state.cpu.io.wire = 10;
    CMP::<ACC>::exec(state);
    assert!(state.cpu.flags.contains(CpuFlags::Zero));
    assert!(state.cpu.flags.contains(CpuFlags::Carry));
    assert!(!state.cpu.flags.contains(CpuFlags::Negative));
}

/// A = A - Memory - Borrow (Borrow = !Carry)
/// Carry is set if resulting borrow is unset
pub struct SBC;
impl MathOp for SBC {
    fn exec(state: &mut State) {
        // Carry is the reverse ("complement") of carry flag.
        let (a_new, borrow) = state.cpu.a.borrowing_sub(state.cpu.io.wire, !state.cpu.flags.contains(CpuFlags::Carry));
        state.cpu.flags.set(CpuFlags::Carry, !borrow);
        // Set negative bit if most sign bit set
        let res_sign = (a_new & 0b1000_0000) != 0;
        state.cpu.flags.set(CpuFlags::Negative, res_sign);

        // Calculate overflow using sign bits.
        let a_sign = state.cpu.a & 0b1000_0000 != 0;
        let bus_sign = state.cpu.io.wire & 0b1000_0000 != 0;
        let overflow = (a_sign ^ bus_sign) & (a_sign ^ res_sign);
        state.cpu.flags.set(CpuFlags::Overflow, overflow);
        // Set zero flag
        state.cpu.flags.set(CpuFlags::Zero, a_new == 0);
        // update accumulator
        state.cpu.a = a_new;
    }
}

/// A = A & Memory
pub struct AND;
impl MathOp for AND {
    fn exec(state: &mut State) {
        // Perform assign AND
        state.cpu.a &= state.cpu.io.wire;
        state.cpu.flags.set(CpuFlags::Negative, (state.cpu.a & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, state.cpu.a == 0);
    }
}

/// A = A | Memory
pub struct ORA;
impl MathOp for ORA {
    fn exec(state: &mut State) {
        // Perform assign OR
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
        state.cpu.flags.set(CpuFlags::Negative, (state.cpu.io.wire & 0b1000_0000) != 0);
        state.cpu.flags.set(CpuFlags::Zero, res == 0);
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
        let reg = reg << 1;
        I::set(state, reg);
        // Flags set accordingly
        state.cpu.flags.set(CpuFlags::Carry, carry);
        state.cpu.flags.set(CpuFlags::Negative, reg & 0b1000_0000 != 0);
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

/// Shift Right a Register ("Logical Shift Right")
pub struct LSR<I: Register>(PhantomData<I>);
impl<I: Register> MathOp for LSR<I> {
    fn exec(state: &mut State) {
        let reg = I::get(state);
        // Carry bit is bit that is shift out
        let carry = (reg & 0b0000_0001) != 0;
        let reg = reg >> 1; // do the shift
        I::set(state, reg); // set the registers
        state.cpu.flags.set(CpuFlags::Carry, carry); // set the carry
        state.cpu.flags.set(CpuFlags::Negative, false); // Negative bit is shifted in, so it is always false
        state.cpu.flags.set(CpuFlags::Zero, reg == 0); // set zero accordingly
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
/// Clear Flag
pub struct CLR<const FLAG: CpuFlags>;
impl<const FLAG: CpuFlags> MathOp for CLR<FLAG> {
    fn exec(state: &mut State) {
        state.cpu.flags.remove(FLAG);
    }
}
/// Set Flag
pub struct SET<const FLAG: CpuFlags>;
impl<const FLAG: CpuFlags> MathOp for SET<FLAG> {
    fn exec(state: &mut State) {
        state.cpu.flags.insert(FLAG);
    }
}

/// Trigger Branch if FLAG matches STATE
pub struct Branch<const FLAG: CpuFlags, const STATE: bool>;
impl<const FLAG: CpuFlags, const STATE: bool> MathOp for Branch<FLAG, STATE> {
    fn exec(state: &mut State) {
        state.cpu.first = Some(state.cpu.io.wire);
        // Check if specific cpu FLAG equals required STATE
        if state.cpu.flags.contains(FLAG) == STATE {
            state.op_state.set(OpState::Branching, true);
            /// If branching, add operand to MEM_LOW, checking for page cross
            IncPC::exec(state);
            SetAddrPC::exec(state);
            AddIndex::<BUS, true>::exec(state); // may cross page
            MV::<MEM_LOW, PCL>::exec(state);
            // println!("{:?}", state);
        } else {
            IncPC::exec(state);
        }
    }
}

/// Set Program Counter from two registers
pub struct SET_PC<L: Register, H: Register>(PhantomData<L>, PhantomData<H>);
impl<L: Register, H: Register> MathOp for SET_PC<L, H> {
    fn exec(state: &mut State) {
        state.cpu.pc_set([L::get(state), H::get(state)]);
    }
}

/// Increase register by one, wrapping. Optionally set CPU flags
pub struct INC<I: Register, const SET_FLAGS: bool = false>(PhantomData<I>);
impl<I: Register, const SET_FLAGS: bool> MathOp for INC<I, SET_FLAGS> {
    fn exec(state: &mut State) {
        let reg = I::get(state).wrapping_add(1);
        I::set(state, reg);
        if SET_FLAGS { SetDefaultFlags::<I>::exec(state); }
    }
}

/// Decrease register by one, wrapping. Optionally set CPU Flags
pub struct DEC<I: Register, const SET_FLAGS: bool = false>(PhantomData<I>);
impl<I: Register, const SET_FLAGS: bool> MathOp for DEC<I, SET_FLAGS> {
    fn exec(state: &mut State) {
        let reg = I::get(state).wrapping_sub(1);
        I::set(state, reg);
        if SET_FLAGS { SetDefaultFlags::<I>::exec(state); }
    }
}

pub struct BRK;
impl MathOp for BRK {
    fn exec(state: &mut State) {
        state.cpu.flags.set(CpuFlags::InterruptDisable, true);
    }
}