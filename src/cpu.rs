
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct CpuFlags: u8 {
        /// Negative flag. Set if the operation output's sign bit is set.
        const Negative = 0b10000000;
        /// Overflow flag. Set an operation overflows in some form.
        const Overflow = 0b01000000;
        const Unused   = 0b00100000;
        /// Break flag. Set in the copy of the Flags register that is pushed on the stack in the course of a BRK instruction.
        /// When RTI (return from interrupt) is performed, this will notify the code that the interrupt was caused internally by BRK as opposed to an external interrupt.
        const Break    = 0b00010000;
        /// Decimal flag
        const Decimal  = 0b00001000;
        /// Zero flag. Set when the output of an operation is zero
        const Zero     = 0b00000010;
        /// Carry flag. used for big-num operations and other things
        const Carry    = 0b00000001;
        /// Prevents the Processor from responding to IRQs (Interrupt Requests). Used in time-sensitive Code
        const InterruptDisable = 0b00000100;
    }
}

/// Derived from: https://www.nesdev.org/wiki/CPU_registers and https://www.nesdev.org/wiki/Status_flags
#[derive(Default, Clone)]
pub struct CPU {
    /// Accumulator Register
    pub a: u8,
    /// X Index Register
    pub x: u8,
    /// Y Index Register
    pub y: u8,
    /// Status Flags
    pub flags: CpuFlags,
    /// Program Counter
    pub pc: u16,
    /// Stack Pointer
    pub sp: u8,
    /// Latch, Temporary storage between cycles of instructions
    pub latch: u8,
    /// First Operand for current instruction
    pub first: Option<u8>,
    /// Second Operand for current instruction
    pub second: Option<u8>,
    /// Effective address that was used for the instruction
    pub eff_addr: Option<u16>,
    pub io: CPUIO,
}
impl std::fmt::Debug for CPU {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpuState")
        .field("a", &format_args!("0x{:X?}", &self.a))
        .field("x", &format_args!("0x{:X?}", &self.x))
        .field("y", &format_args!("0x{:X?}", &self.y))
        .field("flags", &self.flags)
        .field("pc", &format_args!("0x{:X?}", &self.pc))
        .field("sp", &format_args!("0x{:X?}", &self.sp))
        .field("latch", &format_args!("0x{:X?}", &self.latch))
        .field("first", &format_args!("0x{:X?}", &self.first))
        .field("second", &format_args!("0x{:X?}", &self.second))
        .finish()
    }
}
impl CPU {
    /// Get little endian byte array form of program counter
    pub fn pc_get(&self) -> [u8; 2] { self.pc.to_le_bytes() }
    /// Set program counter with little-endian byte array
    pub fn pc_set(&mut self, pc: [u8; 2]) { self.pc = u16::from_le_bytes(pc) }
    pub fn cmp(&self, new: &Self) {
        if self.a != new.a { println!("A: 0x{:X?} -> 0x{:X?}", self.a, new.a) }
        if self.x != new.x { println!("X: 0x{:X?} -> 0x{:X?}", self.x, new.x) }
        if self.y != new.y { println!("Y: 0x{:X?} -> 0x{:X?}", self.y, new.y) }
        if self.flags != new.flags { println!("FLAGS: {:?} -> {:?}", self.flags, new.flags) }
        if self.sp != new.sp { println!("SP: 0x{:X?} -> 0x{:X?}", self.sp, new.sp) }
        if self.latch != new.latch { println!("LATCH: 0x{:X?} -> 0x{:X?}", self.latch, new.latch) }
        if self.pc != new.pc { println!("PC: 0x{:X?} -> 0x{:X?}", self.pc, new.pc) }
    }
}

#[derive(Default, Clone)]
pub struct CPUIO {
    /// Lower 8 bits of address
    pub low: u8,
    /// Upper 8 bits of address
    pub high: u8,
    /// In / Out 8 bits
    pub wire: u8,
}
impl CPUIO {
    pub fn set(&mut self, addr: u16) {
        let bytes = addr.to_le_bytes();
        self.low = bytes[0];
        self.high = bytes[1];
    }
}
impl std::fmt::Debug for CPUIO {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryBus")
        .field("low", &format_args!("0x{:X?}", &self.low))
        .field("high", &format_args!("0x{:X?}", &self.high))
        .field("wire", &format_args!("0x{:X?}", &self.wire))
        .finish()
    }
}