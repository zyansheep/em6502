pub use super::*;

pub const INSTR_SET: [(&'static str, &'static [fn(&mut State)]); 256] = [
	("BRK",			&[read_byte::<PCRead>, push_stack::<PCH>, push_stack::<PCL>, push_stack::<FLAGS_WITH_BREAK>, read_to_reg::<ConstRead<0xFFFE>, PCL>, read_high_reg_low::<ConstRead<0xFFFF>, PCL, BRK>]), // x00
	("ORA ($nn,X)",	&indexed_indirect(read_op::<ORA>())), // x01
	("*KIL",		&[]), // 02
	("*SLO",		&[]), // 03
	("*NOP",		&[]), // 04
	("ORA $nn",		&zeropage(read_op::<ORA>())), // x05
	("ASL $nn",		&zeropage(rw_op::<ASL<BUS>>())), // x06
	("*SLO",		&[]), // 07
	("PHP",			&[read_byte::<RegRead>, push_stack::<FLAGS>]), // x08
	("ORA #$nn",	&immediate::<ORA>()), // x09
	("ASL A",		&implied::<ASL<Acc>>()), // x0A
	("ANC",			&[]), // 0B
	("*NOP",		&[]), // 0C
	("ORA $nnnn",	&absolute(read_op::<ORA>())), // x0D
	("ASL $nnnn",	&absolute(rw_op::<ASL<BUS>>())), // x0E
	("*SLO",		&[]), // 0F
	("BPL $nn",		&relative::<Branch<{CpuFlags::Negative}, false>>()), // x10
	("ORA ($nn),Y",	&indirect_indexed(read_op::<ORA>())), // x11
	("*KIL",		&[]), // 12
	("*SLO",		&[]), // 13
	("*NOP",		&[]), // 14
	("ORA $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<ORA>())), // x15
	("ASL $nn,X",	&indexed_indirect(rw_op::<ASL<BUS>>())), // x16
	("*SLO",		&[]), // 17
	("CLC",			&implied::<CL<{CpuFlags::Carry}>>()), // x18
	("ORA $nnnn,Y",	&absolute_indexed::<YIndex, _>(read_op::<ORA>())), // x19
	("*NOP",		&implied::<NOP>()), // x1A
	("*SLO",		&[]), // 1B
	("*NOP",		&[]), // 1C
	("ORA $nnnn,X",	&absolute_indexed::<XIndex, _>(read_op::<ORA>())), // x1D
	("ASL $nnnn,X",	&absolute_indexed::<XIndex, _>(rw_op::<ASL<BUS>>())), // x1E
	("*SLO",		&[]), // 1F
	("JSR",			&[read_to_reg::<PCRead, LATCH>, run::<DEC<STACK_POINTER>>, push_stack::<PCH>, push_stack::<PCL>, read_high_reg_low::<PCRead, LATCH, JMP>]), // x20
	("AND ($nn,X)",	&indexed_indirect(read_op::<AND>())), // x21
	("*KIL",		&[]), // 22
	("*RLA",		&[]), // 23
	("BIT $nn",		&zeropage(read_op::<BIT>())), // x24
	("AND $nn",		&zeropage(read_op::<AND>())), // x25
	("ROL $nn",		&zeropage(read_op::<ROL<BUS>>())), // x26
	("*RLA $nn",	&[]), // x27
	("PLP",			&[pop_stack::<FLAGS>]), // x28
	("AND #$nn",	&immediate::<AND>()), // x29
	("ROL A",		&implied::<ROL<Acc>>()), // x2A
	("ANC",			&[]), // 2B
	("BIT $nnnn",	&absolute(read_op::<BIT>())), // x2C
	("AND $nnnn",	&absolute(read_op::<AND>())), // x2D
	("ROL $nnnn",	&absolute(rw_op::<ROL<BUS>>())), // x2E
	("*RLA",		&[]), // 2F
	("BMI $nn",		&relative::<Branch<{CpuFlags::Negative}, true>>()), // x30
	("AND ($nn),Y",	&indirect_indexed(read_op::<AND>())), // x31
	("*KIL",		&[]), // 32
	("*RLA",		&[]), // 33
	("*NOP $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<NOP>())), // x34
	("AND $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<AND>())), // x35
	("ROL $nn,X",	&indexed_indirect(rw_op::<ROL<BUS>>())), // x36
	("*RLA",		&[]), // 37
	("SEC",			&implied::<CL<{CpuFlags::Carry}>>()), // x38
	("AND $nnnn,Y",	&absolute_indexed::<YIndex, _>(read_op::<AND>())), // x39
	("*NOP",		&implied::<NOP>()), // x3A
	("*RLA",		&[]), // 3B
	("*NOP $nnnn,X",&absolute_indexed::<XIndex, _>(read_op::<NOP>())), // x3C
	("AND $nnnn,X",	&absolute_indexed::<XIndex, _>(read_op::<AND>())), // x3D
	("ROL $nnnn,X",	&absolute_indexed::<XIndex, _>(rw_op::<ROL<BUS>>())), // x3E
	("*RLA",		&[]), // 3F
	("RTI",			&[read_byte::<PCRead>, run::<INC<STACK_POINTER>>, pop_stack::<FLAGS>, pop_stack::<PCL>, pop_stack::<PCH>]), // x40
	("EOR ($nn,X)",	&indexed_indirect(read_op::<EOR>())), // x41
	("*KIL",		&[]), // 42
	("SRE",			&[]), // 43
	("*NOP $nn",	&zeropage(read_op::<NOP>())), // x44
	("EOR $nn",		&zeropage(read_op::<EOR>())), // x45
	("LSR $nn",		&zeropage(rw_op::<LSR<BUS>>())), // x46
	("SRE",			&[]), // 47
	("PHA",			&[read_byte::<RegRead>, push_stack::<Acc>]), // x48
	("EOR #$nn",	&immediate::<EOR>()), // x49
	("LSR A",		&implied::<LSR<Acc>>()), // x4A
	("ALR",			&[]), // 4B
	("JMP $nnnn",	&absolute(implied::<JMP>())), // x4C
	("EOR $nnnn",	&absolute(read_op::<EOR>())), // x4D
	("LSR $nnnn",	&absolute(rw_op::<LSR<BUS>>())), // x4E
	("SRE",			&[]), // 4F
	("BVC $nn",		&relative::<Branch<{CpuFlags::Overflow}, false>>()), // x50
	("EOR ($nn),Y",	&indirect_indexed(read_op::<EOR>())), // x51
	("*KIL",		&[]), // 52
	("SRE",			&[]), // 53
	("*NOP $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<NOP>())), // x54
	("EOR $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<EOR>())), // x55
	("LSR $nn,X",	&indexed_indirect(rw_op::<LSR<BUS>>())), // x56
	("SRE",			&[]), // 57
	("CLI",			&implied::<CL<{CpuFlags::InterruptDisable}>>()), // x58
	("EOR $nnnn,Y",	&absolute_indexed::<YIndex, _>(read_op::<EOR>())), // x59
	("*NOP",		&implied::<NOP>()), // x5A
	("SRE",			&[]), // 5B
	("*NOP $nnnn,X",&absolute_indexed::<XIndex, _>(read_op::<NOP>())), // x5C
	("EOR $nnnn,X",	&absolute_indexed::<XIndex, _>(read_op::<EOR>())), // x5D
	("LSR $nnnn,X",	&absolute_indexed::<XIndex, _>(rw_op::<LSR<BUS>>())), // x5E
	("SRE",			&[]), // 5F
	("RTS",			&[read_byte::<PCRead>, pop_stack::<BUS>, pop_stack::<PCL>, pop_stack::<PCH>]), // x60
	("ADC ($nn,X)",	&indexed_indirect(read_op::<ADC>())), // x61
	("*KIL",		&[]), // 62
	("RRA",			&[]), // 63
	("*NOP $nn",	&zeropage(read_op::<NOP>())), // x64
	("ADC $nn",		&zeropage(read_op::<ADC>())), // x65
	("ROR $nn",		&zeropage(rw_op::<ROR<BUS>>())), // x66
	("RRA",			&[]), // 67
	("PLA",			&[read_byte::<RegRead>, pop_stack::<Acc>]), // x68
	("ADC #$nn",	&immediate::<ADC>()), // x69
	("ROR A",		&implied::<ROR<Acc>>()), // x6A
	("ARR",			&[]), // 6B
	("JMP ($nnnn)",	&absolute_indirect::<JMP>()), // x6C
	("ADC $nnnn",	&absolute(read_op::<ADC>())), // x6D
	("ROR $nnnn",	&absolute(rw_op::<ROR<BUS>>())), // x6E
	("RRA",			&[]), // 6F
	("BVS $nn",		&relative::<Branch<{CpuFlags::Overflow}, true>>()), // x70
	("ADC ($nn),Y",	&indirect_indexed(read_op::<ADC>())), // x71
	("*KIL",		&[]), // 72
	("RRA",			&[]), // 73
	("*NOP $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<NOP>())), // x74
	("ADC $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<ADC>())), // x75
	("ROR $nn,X",	&indexed_indirect(rw_op::<ROR<BUS>>())), // x76
	("*RRA",		&[]), // 77
	("SEI",			&implied::<SET<{CpuFlags::InterruptDisable}>>()), // x78
	("ADC $nnnn,Y",	&absolute_indexed::<YIndex, _>(read_op::<ADC>())), // x79
	("*NOP",		&implied::<NOP>()), // x7A
	("RRA $nnnn,Y",	&[]), // 7B
	("*NOP $nnnn,X",&absolute_indexed::<XIndex, _>(read_op::<NOP>())), // x7C
	("ADC $nnnn,X",	&absolute_indexed::<XIndex, _>(read_op::<ADC>())), // x7D
	("ROR $nnnn,X",	&absolute_indexed::<XIndex, _>(rw_op::<ROR<BUS>>())), // x7E
	("*RRA $nnnn,Y",&[]), // 7F
	("*NOP",		&immediate::<NOP>()), // x80
	("STA ($nn,X)",	&indexed_indirect(write_op::<ST<Acc>>())), // x81
	("*KIL",		&[]), // 82
	("SAX",			&[]), // 83
	("STY $nn",		&zeropage(write_op::<ST<YIndex>>())), // x84
	("STA $nn",		&zeropage(write_op::<ST<Acc>>())), // x85
	("STX $nn",		&zeropage(write_op::<ST<XIndex>>())), // x86
	("SAX",			&[]), // 87
	("DEY",			&implied::<DEC<YIndex>>()), // x88
	("*NOP",		&[]), // x89
	("TXA",			&implied::<TR<XIndex, Acc>>()), // x8A
	("XAA",			&[]), // 8B
	("STY $nnnn",	&absolute(write_op::<ST<YIndex>>())), // x8C
	("STA $nnnn",	&absolute(write_op::<ST<Acc>>())), // x8D
	("STX $nnnn",	&absolute(write_op::<ST<XIndex>>())), // x8E
	("SAX",			&[]), // 8F
	("BCC $nn",		&relative::<Branch<{CpuFlags::Carry}, false>>()), // x90
	("STA ($nn),Y",	&indirect_indexed(write_op::<ST<Acc>>())), // x91
	("*KIL",		&[]), // 92
	("AHX",			&[]), // 93
	("STY $nn,X",	&zeropage_indexed::<XIndex, _>(write_op::<ST<YIndex>>())), // x94
	("STA $nn,X",	&zeropage_indexed::<XIndex, _>(write_op::<ST<Acc>>())), // x95
	("STX $nn,Y",	&zeropage_indexed::<YIndex, _>(write_op::<ST<XIndex>>())), // x96
	("SAX",			&[]), // 97
	("TYA",			&implied::<TR<YIndex, Acc>>()), // x98
	("STA $nnnn,Y",	&absolute_indexed::<YIndex, _>(write_op::<ST<Acc>>())), // x99
	("TXS",			&implied::<TR<XIndex, STACK_POINTER>>()), // x9A
	("TAS",			&[]), // 9B
	("SHY",			&[]), // 9C
	("STA $nnnn,X",	&absolute_indexed::<XIndex, _>(write_op::<ST<Acc>>())), // x9D
	("SHX",			&[]), // 9E
	("AHX",			&[]), // 9F
	("LDY #$nn",	&immediate::<LD<YIndex>>()), // xA0
	("LDA ($nn,X)",	&indexed_indirect(read_op::<LD<Acc>>())), // xA1
	("LDX #$nn",	&immediate::<LD<XIndex>>()), // xA2
	("LAX",			&[]), // xA3
	("LDY $nn",		&zeropage(read_op::<LD<YIndex>>())), // xA4
	("LDA $nn",		&zeropage(read_op::<LD<Acc>>())), // xA5
	("LDX $nn",		&zeropage(read_op::<LD<XIndex>>())), // xA6
	("LAX",			&[]), // xA7
	("TAY",			&implied::<TR<Acc, YIndex>>()), // xA8
	("LDA #$nn",	&immediate::<LD<Acc>>()), // xA9
	("TAX",			&implied::<TR<Acc, XIndex>>()), // xAA
	("LAX",			&[]), // xAB
	("LDY $nnnn",	&absolute(read_op::<LD<YIndex>>())), // xAC
	("LDA $nnnn",	&absolute(read_op::<LD<Acc>>())), // xAD
	("LDX $nnnn",	&absolute(read_op::<LD<XIndex>>())), // xAE
	("LAX",			&[]), // xAF
	("BCS $nn",		&relative::<Branch<{CpuFlags::Carry}, true>>()), // xB0
	("LDA ($nn),Y",	&indirect_indexed(read_op::<LD<Acc>>())), // xB1
	("*KIL",		&[]), // xB2
	("LAX ($nn),Y",	&[]), // xB3
	("LDY $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<LD<YIndex>>())), // xB4
	("LDA $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<LD<Acc>>())), // xB5
	("LDX $nn,Y",	&zeropage_indexed::<YIndex, _>(read_op::<LD<XIndex>>())), // xB6
	("LAX",			&[]), // xB7
	("CLV",			&implied::<CL<{CpuFlags::Overflow}>>()), // xB8
	("LDA $nnnn,Y",	&absolute_indexed::<YIndex, _>(read_op::<LD<Acc>>())), // xB9
	("TSX",			&implied::<TR<STACK_POINTER, XIndex>>()), // xBA
	("LAS $nnnn,Y",	&[]), // xBB
	("LDY $nnnn,X",	&absolute_indexed::<XIndex, _>(read_op::<LD<YIndex>>())), // xBC
	("LDA $nnnn,X",	&absolute_indexed::<XIndex, _>(read_op::<LD<Acc>>())), // xBD
	("LDX $nnnn,Y",	&absolute_indexed::<YIndex, _>(read_op::<LD<XIndex>>())), // xBE
	("LAX",			&[]), // xBF
	("CPY #$nn",	&immediate::<CMP<Acc>>()), // xC0
	("CMP ($nn,X)",	&indexed_indirect(read_op::<CMP<Acc>>())), // xC1
	("*NOP",		&[]), // xC2
	("*DCP",		&[]), // xC3
	("CPY $nn",		&zeropage(read_op::<CMP<YIndex>>())), // xC4
	("CMP $nn",		&zeropage(read_op::<CMP<Acc>>())), // xC5
	("DEC $nn",		&zeropage(rw_op::<DEC<BUS>>())), // xC6
	("*DCP",		&[]), // xC7
	("INY",			&implied::<INC<YIndex>>()), // xC8
	("CMP #$nn",	&immediate::<CMP<Acc>>()), // xC9
	("DEX",			&implied::<INC<XIndex>>()), // xCA
	("AXS",			&[]), // xCB
	("CPY $nnnn",	&absolute(read_op::<CMP<YIndex>>())), // xCC
	("CMP $nnnn",	&absolute(read_op::<CMP<Acc>>())), // xCD
	("DEC $nnnn",	&absolute(rw_op::<DEC<BUS>>())), // xCE
	("*DCP",		&[]), // xCF
	("BNE $nn",		&relative::<Branch<{CpuFlags::Zero}, false>>()), // xD0
	("CMP ($nn),Y",	&indirect_indexed(read_op::<CMP<Acc>>())), // xD1
	("*KIL",		&[]), // xD2
	("*DCP ($nn),Y",&[]), // xD3
	("*NOP $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<NOP>())), // xD4
	("CMP $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<CMP<Acc>>())), // xD5
	("DEC $nn,X",	&zeropage_indexed::<XIndex, _>(rw_op::<DEC<BUS>>())), // xD6
	("*DCP",		&[]), // xD7
	("CLD",			&implied::<CL<{CpuFlags::Decimal}>>()), // xD8
	("CMP $nnnn,Y",	&absolute_indexed::<YIndex, _>(read_op::<CMP<Acc>>())), // xD9
	("*NOP",		&implied::<NOP>()), // xDA
	("*DCP",		&[]), // xDB
	("*NOP $nnnn,X",&absolute_indexed::<XIndex, _>(read_op::<NOP>())), // xDC
	("CMP $nnnn,X",	&absolute_indexed::<XIndex, _>(read_op::<CMP<Acc>>())), // xDD
	("DEC $nnnn,X",	&absolute_indexed::<XIndex, _>(rw_op::<DEC<BUS>>())), // xDE
	("*DCP",		&[]), // xDF
	("CPX #$nn",	&immediate::<CMP<Acc>>()), // xE0
	("SBC ($nn,X)",	&indexed_indirect(read_op::<SBC>())), // xE1
	("*NOP",		&[]), // xE2
	("*ISC $nn",	&[]), // xE3
	("CPX $nn",		&zeropage(read_op::<CMP<Acc>>())), // xE4
	("SBC $nn",		&zeropage(read_op::<SBC>())), // xE5
	("INC $nn",		&zeropage(rw_op::<INC<BUS>>())), // xE6
	("*ISC",		&[]), // xE7
	("INX",			&implied::<INC<Acc>>()), // xE8
	("SBC #$nn",	&immediate::<SBC>()), // xE9
	("NOP",			&implied::<NOP>()), // xEA
	("SBC",			&[]), // xEB
	("CPX $nnnn",	&absolute(read_op::<CMP<Acc>>())), // xEC
	("SBC $nnnn",	&absolute(read_op::<SBC>())), // xED
	("INC $nnnn",	&absolute(rw_op::<INC<BUS>>())), // xEE
	("*ISC",		&[]), // xEF
	("BEQ $nn",		&relative::<Branch<{CpuFlags::Zero}, true>>()), // xF0
	("SBC ($nn),Y",	&indirect_indexed(read_op::<SBC>())), // xF1
	("*KIL",		&[]), // xF2
	("*ISC",		&[]), // xF3
	("*NOP $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<NOP>())), // xF4
	("SBC $nn,X",	&zeropage_indexed::<XIndex, _>(read_op::<SBC>())), // xF5
	("INC $nn,X",	&zeropage_indexed::<XIndex, _>(rw_op::<INC<BUS>>())), // xF6
	("*ISC",		&[]), // xF7
	("SED",			&implied::<SET<{CpuFlags::Decimal}>>()), // xF8
	("SBC $nnnn,Y",	&absolute_indexed::<YIndex, _>(read_op::<SBC>())), // xF9
	("*NOP",		&implied::<NOP>()), // xFA
	("*ISC",		&[]), // xFB
	("*NOP $nnnn,X",&absolute_indexed::<XIndex, _>(read_op::<NOP>())), // xFC
	("SBC $nnnn,X",	&absolute_indexed::<XIndex, _>(read_op::<SBC>())), // xFD
	("INC $nnnn,X",	&absolute_indexed::<XIndex, _>(rw_op::<INC<BUS>>())), // xFE
	("*ISC",		&[]), // xFF
];
