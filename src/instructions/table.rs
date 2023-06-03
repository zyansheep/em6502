pub use super::*;

pub const INSTR_SET: [(&'static str, &'static [fn(&mut State)]); 256] = [
	("BRK",			&BRK), // 00
	("ORA ($nn,X)",	&indexed_indirect(read_op::<ORA>())), // 01
	("*KIL",		&[]), // 02
	("*SLO",		&[]), // 03
	("*NOP",		&[]), // 04
	("ORA $nn",		&zeropage(read_op::<ORA>())), // 05
	("ASL $nn",		&zeropage(rw_op::<ASL<BUS>>())), // 06
	("*SLO",		&[]), // 07
	("PHP",			&push_stack::<FLAGS_REMOVE_BREAK>()), // 08
	("ORA #$nn",	&immediate::<ORA>()), // 09
	("ASL A",		&implied::<ASL<ACC>>()), // 0A
	("ANC",			&[]), // 0B
	("*NOP",		&[]), // 0C
	("ORA $nnnn",	&absolute(read_op::<ORA>())), // 0D
	("ASL $nnnn",	&absolute(rw_op::<ASL<BUS>>())), // 0E
	("*SLO",		&[]), // 0F
	("BPL $nn",		&branch_if::<{CpuFlags::Negative}, false>()), // 10
	("ORA ($nn),Y",	&indirect_indexed(read_op::<ORA>())), // 11
	("*KIL",		&[]), // 12
	("*SLO",		&[]), // 13
	("*NOP",		&[]), // 14
	("ORA $nn,X",	&zeropage_indexed::<X, _>(read_op::<ORA>())), // 15
	("ASL $nn,X",	&indexed_indirect(rw_op::<ASL<BUS>>())), // 16
	("*SLO",		&[]), // 17
	("CLC",			&implied::<CLR<{CpuFlags::Carry}>>()), // 18
	("ORA $nnnn,Y",	&absolute_indexed::<Y, _>(read_op::<ORA>())), // 19
	("*NOP",		&implied::<NOP>()), // 1A
	("*SLO",		&[]), // 1B
	("*NOP",		&[]), // 1C
	("ORA $nnnn,X",	&absolute_indexed::<X, _>(read_op::<ORA>())), // 1D
	("ASL $nnnn,X",	&absolute_indexed::<X, _>(rw_op::<ASL<BUS>>())), // 1E
	("*SLO",		&[]), // 1F
	("JSR",			&JSR), // 20
	("AND ($nn,X)",	&indexed_indirect(read_op::<AND>())), // 21
	("*KIL",		&[]), // 22
	("*RLA",		&[]), // 23
	("BIT $nn",		&zeropage(read_op::<BIT>())), // 24
	("AND $nn",		&zeropage(read_op::<AND>())), // 25
	("ROL $nn",		&zeropage(read_op::<ROL<BUS>>())), // 26
	("*RLA $nn",	&[]), // 27
	("PLP",			&pull_stack::<FLAGS_REMOVE_BREAK, NOP>()), // 28
	("AND #$nn",	&immediate::<AND>()), // 29
	("ROL A",		&implied::<ROL<ACC>>()), // 2A
	("ANC",			&[]), // 2B
	("BIT $nnnn",	&absolute(read_op::<BIT>())), // 2C
	("AND $nnnn",	&absolute(read_op::<AND>())), // 2D
	("ROL $nnnn",	&absolute(rw_op::<ROL<BUS>>())), // 2E
	("*RLA",		&[]), // 2F
	("BMI $nn",		&branch_if::<{CpuFlags::Negative}, true>()), // 30
	("AND ($nn),Y",	&indirect_indexed(read_op::<AND>())), // 31
	("*KIL",		&[]), // 32
	("*RLA",		&[]), // 33
	("*NOP $nn,X",	&zeropage_indexed::<X, _>(read_op::<NOP>())), // 34
	("AND $nn,X",	&zeropage_indexed::<X, _>(read_op::<AND>())), // 35
	("ROL $nn,X",	&indexed_indirect(rw_op::<ROL<BUS>>())), // 36
	("*RLA",		&[]), // 37
	("SEC",			&implied::<SET<{CpuFlags::Carry}>>()), // 38
	("AND $nnnn,Y",	&absolute_indexed::<Y, _>(read_op::<AND>())), // 39
	("*NOP",		&implied::<NOP>()), // 3A
	("*RLA",		&[]), // 3B
	("*NOP $nnnn,X",&absolute_indexed::<X, _>(read_op::<NOP>())), // 3C
	("AND $nnnn,X",	&absolute_indexed::<X, _>(read_op::<AND>())), // 3D
	("ROL $nnnn,X",	&absolute_indexed::<X, _>(rw_op::<ROL<BUS>>())), // 3E
	("*RLA",		&[]), // 3F
	("RTI",			&RTI), // 40
	("EOR ($nn,X)",	&indexed_indirect(read_op::<EOR>())), // 41
	("*KIL",		&[]), // 42
	("SRE",			&[]), // 43
	("*NOP $nn",	&zeropage(read_op::<NOP>())), // 44
	("EOR $nn",		&zeropage(read_op::<EOR>())), // 45
	("LSR $nn",		&zeropage(rw_op::<LSR<BUS>>())), // 46
	("SRE",			&[]), // 47
	("PHA",			&push_stack::<ACC>()), // 48
	("EOR #$nn",	&immediate::<EOR>()), // 49
	("LSR A",		&implied::<LSR<ACC>>()), // 4A
	("ALR",			&[]), // 4B
	("JMP $nnnn",	&[read::<SetAddrPC, ReadFirst>, read::<SetAddrPC, Seq<ReadSecond, SET_PC<FIRST, SECOND>>>]), // 4C
	("EOR $nnnn",	&absolute(read_op::<EOR>())), // 4D
	("LSR $nnnn",	&absolute(rw_op::<LSR<BUS>>())), // 4E
	("SRE",			&[]), // 4F
	("BVC $nn",		&branch_if::<{CpuFlags::Overflow}, false>()), // 50
	("EOR ($nn),Y",	&indirect_indexed(read_op::<EOR>())), // 51
	("*KIL",		&[]), // 52
	("SRE",			&[]), // 53
	("*NOP $nn,X",	&zeropage_indexed::<X, _>(read_op::<NOP>())), // 54
	("EOR $nn,X",	&zeropage_indexed::<X, _>(read_op::<EOR>())), // 55
	("LSR $nn,X",	&indexed_indirect(rw_op::<LSR<BUS>>())), // 56
	("SRE",			&[]), // 57
	("CLI",			&implied::<CLR<{CpuFlags::InterruptDisable}>>()), // 58
	("EOR $nnnn,Y",	&absolute_indexed::<Y, _>(read_op::<EOR>())), // 59
	("*NOP",		&implied::<NOP>()), // 5A
	("SRE",			&[]), // 5B
	("*NOP $nnnn,X",&absolute_indexed::<X, _>(read_op::<NOP>())), // 5C
	("EOR $nnnn,X",	&absolute_indexed::<X, _>(read_op::<EOR>())), // 5D
	("LSR $nnnn,X",	&absolute_indexed::<X, _>(rw_op::<LSR<BUS>>())), // 5E
	("SRE",			&[]), // 5F
	("RTS",			&RTS), // 60
	("ADC ($nn,X)",	&indexed_indirect(read_op::<ADC>())), // 61
	("*KIL",		&[]), // 62
	("RRA",			&[]), // 63
	("*NOP $nn",	&zeropage(read_op::<NOP>())), // 64
	("ADC $nn",		&zeropage(read_op::<ADC>())), // 65
	("ROR $nn",		&zeropage(rw_op::<ROR<BUS>>())), // 66
	("RRA",			&[]), // 67
	("PLA",			&pull_stack::<ACC, SetDefaultFlags<ACC>>()), // 68
	("ADC #$nn",	&immediate::<ADC>()), // 69
	("ROR A",		&implied::<ROR<ACC>>()), // 6A
	("ARR",			&[]), // 6B
	("JMP ($nnnn)",	&absolute_indirect_jmp()), // 6C
	("ADC $nnnn",	&absolute(read_op::<ADC>())), // 6D
	("ROR $nnnn",	&absolute(rw_op::<ROR<BUS>>())), // 6E
	("RRA",			&[]), // 6F
	("BVS $nn",		&branch_if::<{CpuFlags::Overflow}, true>()), // 70
	("ADC ($nn),Y",	&indirect_indexed(read_op::<ADC>())), // 71
	("*KIL",		&[]), // 72
	("RRA",			&[]), // 73
	("*NOP $nn,X",	&zeropage_indexed::<X, _>(read_op::<NOP>())), // 74
	("ADC $nn,X",	&zeropage_indexed::<X, _>(read_op::<ADC>())), // 75
	("ROR $nn,X",	&indexed_indirect(rw_op::<ROR<BUS>>())), // 76
	("*RRA",		&[]), // 77
	("SEI",			&implied::<SET<{CpuFlags::InterruptDisable}>>()), // 78
	("ADC $nnnn,Y",	&absolute_indexed::<Y, _>(read_op::<ADC>())), // 79
	("*NOP",		&implied::<NOP>()), // 7A
	("RRA $nnnn,Y",	&[]), // 7B
	("*NOP $nnnn,X",&absolute_indexed::<X, _>(read_op::<NOP>())), // 7C
	("ADC $nnnn,X",	&absolute_indexed::<X, _>(read_op::<ADC>())), // 7D
	("ROR $nnnn,X",	&absolute_indexed::<X, _>(rw_op::<ROR<BUS>>())), // 7E
	("*RRA $nnnn,Y",&[]), // 7F
	("*NOP",		&immediate::<NOP>()), // 80
	("STA ($nn,X)",	&indexed_indirect(write_op::<ST<ACC>>())), // 81
	("*KIL",		&[]), // 82
	("SAX",			&[]), // 83
	("STY $nn",		&zeropage(write_op::<ST<Y>>())), // 84
	("STA $nn",		&zeropage(write_op::<ST<ACC>>())), // 85
	("STX $nn",		&zeropage(write_op::<ST<X>>())), // 86
	("SAX",			&[]), // 87
	("DEY",			&implied::<DEC<Y>>()), // 88
	("*NOP",		&[]), // 89
	("TXA",			&implied::<MV<X, ACC>>()), // 8A
	("XAA",			&[]), // 8B
	("STY $nnnn",	&absolute(write_op::<ST<Y>>())), // 8C
	("STA $nnnn",	&absolute(write_op::<ST<ACC>>())), // 8D
	("STX $nnnn",	&absolute(write_op::<ST<X>>())), // 8E
	("SAX",			&[]), // 8F
	("BCC $nn",		&branch_if::<{CpuFlags::Carry}, false>()), // 90
	("STA ($nn),Y",	&indirect_indexed(write_op::<ST<ACC>>())), // 91
	("*KIL",		&[]), // 92
	("AHX",			&[]), // 93
	("STY $nn,X",	&zeropage_indexed::<X, _>(write_op::<ST<Y>>())), // 94
	("STA $nn,X",	&zeropage_indexed::<X, _>(write_op::<ST<ACC>>())), // 95
	("STX $nn,Y",	&zeropage_indexed::<Y, _>(write_op::<ST<X>>())), // 96
	("SAX",			&[]), // 97
	("TYA",			&implied::<MV<Y, ACC>>()), // 98
	("STA $nnnn,Y",	&absolute_indexed::<Y, _>(write_op::<ST<ACC>>())), // 99
	("TXS",			&implied::<MV<X, SP>>()), // 9A
	("TAS",			&[]), // 9B
	("SHY",			&[]), // 9C
	("STA $nnnn,X",	&absolute_indexed::<X, _>(write_op::<ST<ACC>>())), // 9D
	("SHX",			&[]), // 9E
	("AHX",			&[]), // 9F
	("LDY #$nn",	&immediate::<LDF<Y>>()), // A0
	("LDA ($nn,X)",	&indexed_indirect(read_op::<LDF<ACC>>())), // A1
	("LDX #$nn",	&immediate::<LDF<X>>()), // A2
	("LAX",			&[]), // A3
	("LDY $nn",		&zeropage(read_op::<LDF<Y>>())), // A4
	("LDA $nn",		&zeropage(read_op::<LDF<ACC>>())), // A5
	("LDX $nn",		&zeropage(read_op::<LDF<X>>())), // A6
	("LAX",			&[]), // A7
	("TAY",			&implied::<MV<ACC, Y>>()), // A8
	("LDA #$nn",	&immediate::<LDF<ACC>>()), // A9
	("TAX",			&implied::<MV<ACC, X>>()), // AA
	("LAX",			&[]), // AB
	("LDY $nnnn",	&absolute(read_op::<LDF<Y>>())), // AC
	("LDA $nnnn",	&absolute(read_op::<LDF<ACC>>())), // AD
	("LDX $nnnn",	&absolute(read_op::<LDF<X>>())), // AE
	("LAX",			&[]), // AF
	("BCS $nn",		&branch_if::<{CpuFlags::Carry}, true>()), // B0
	("LDA ($nn),Y",	&indirect_indexed(read_op::<LDF<ACC>>())), // B1
	("*KIL",		&[]), // B2
	("LAX ($nn),Y",	&[]), // B3
	("LDY $nn,X",	&zeropage_indexed::<X, _>(read_op::<LDF<Y>>())), // B4
	("LDA $nn,X",	&zeropage_indexed::<X, _>(read_op::<LDF<ACC>>())), // B5
	("LDX $nn,Y",	&zeropage_indexed::<Y, _>(read_op::<LDF<X>>())), // B6
	("LAX",			&[]), // B7
	("CLV",			&implied::<CLR<{CpuFlags::Overflow}>>()), // B8
	("LDA $nnnn,Y",	&absolute_indexed::<Y, _>(read_op::<LDF<ACC>>())), // B9
	("TSX",			&implied::<MV<SP, X>>()), // BA
	("LAS $nnnn,Y",	&[]), // BB
	("LDY $nnnn,X",	&absolute_indexed::<X, _>(read_op::<LDF<Y>>())), // BC
	("LDA $nnnn,X",	&absolute_indexed::<X, _>(read_op::<LDF<ACC>>())), // BD
	("LDX $nnnn,Y",	&absolute_indexed::<Y, _>(read_op::<LDF<X>>())), // BE
	("LAX",			&[]), // BF
	("CPY #$nn",	&immediate::<CMP<Y>>()), // C0
	("CMP ($nn,X)",	&indexed_indirect(read_op::<CMP<ACC>>())), // C1
	("*NOP",		&[]), // C2
	("*DCP",		&[]), // C3
	("CPY $nn",		&zeropage(read_op::<CMP<Y>>())), // C4
	("CMP $nn",		&zeropage(read_op::<CMP<ACC>>())), // C5
	("DEC $nn",		&zeropage(rw_op::<DEC<BUS>>())), // C6
	("*DCP",		&[]), // C7
	("INY",			&implied::<INC<Y, true>>()), // C8
	("CMP #$nn",	&immediate::<CMP<ACC>>()), // C9
	("DEX",			&implied::<INC<X, true>>()), // CA
	("AXS",			&[]), // CB
	("CPY $nnnn",	&absolute(read_op::<CMP<Y>>())), // CC
	("CMP $nnnn",	&absolute(read_op::<CMP<ACC>>())), // CD
	("DEC $nnnn",	&absolute(rw_op::<DEC<BUS>>())), // CE
	("*DCP",		&[]), // CF
	("BNE $nn",		&branch_if::<{CpuFlags::Zero}, false>()), // D0
	("CMP ($nn),Y",	&indirect_indexed(read_op::<CMP<ACC>>())), // D1
	("*KIL",		&[]), // D2
	("*DCP ($nn),Y",&[]), // D3
	("*NOP $nn,X",	&zeropage_indexed::<X, _>(read_op::<NOP>())), // D4
	("CMP $nn,X",	&zeropage_indexed::<X, _>(read_op::<CMP<ACC>>())), // D5
	("DEC $nn,X",	&zeropage_indexed::<X, _>(rw_op::<DEC<BUS>>())), // D6
	("*DCP",		&[]), // D7
	("CLD",			&implied::<CLR<{CpuFlags::Decimal}>>()), // D8
	("CMP $nnnn,Y",	&absolute_indexed::<Y, _>(read_op::<CMP<ACC>>())), // D9
	("*NOP",		&implied::<NOP>()), // DA
	("*DCP",		&[]), // DB
	("*NOP $nnnn,X",&absolute_indexed::<X, _>(read_op::<NOP>())), // DC
	("CMP $nnnn,X",	&absolute_indexed::<X, _>(read_op::<CMP<ACC>>())), // DD
	("DEC $nnnn,X",	&absolute_indexed::<X, _>(rw_op::<DEC<BUS>>())), // DE
	("*DCP",		&[]), // DF
	("CPX #$nn",	&immediate::<CMP<X>>()), // E0
	("SBC ($nn,X)",	&indexed_indirect(read_op::<SBC>())), // E1
	("*NOP",		&[]), // E2
	("*ISC $nn",	&[]), // E3
	("CPX $nn",		&zeropage(read_op::<CMP<X>>())), // E4
	("SBC $nn",		&zeropage(read_op::<SBC>())), // E5
	("INC $nn",		&zeropage(rw_op::<INC<BUS, true>>())), // E6
	("*ISC",		&[]), // E7
	("INX",			&implied::<INC<ACC, true>>()), // E8
	("SBC #$nn",	&immediate::<SBC>()), // E9
	("NOP",			&implied::<NOP>()), // EA
	("SBC",			&[]), // EB
	("CPX $nnnn",	&absolute(read_op::<CMP<X>>())), // EC
	("SBC $nnnn",	&absolute(read_op::<SBC>())), // ED
	("INC $nnnn",	&absolute(rw_op::<INC<BUS, true>>())), // EE
	("*ISC",		&[]), // EF
	("BEQ $nn",		&branch_if::<{CpuFlags::Zero}, true>()), // F0
	("SBC ($nn),Y",	&indirect_indexed(read_op::<SBC>())), // F1
	("*KIL",		&[]), // F2
	("*ISC",		&[]), // F3
	("*NOP $nn,X",	&zeropage_indexed::<X, _>(read_op::<NOP>())), // F4
	("SBC $nn,X",	&zeropage_indexed::<X, _>(read_op::<SBC>())), // F5
	("INC $nn,X",	&zeropage_indexed::<X, _>(rw_op::<INC<BUS, true>>())), // F6
	("*ISC",		&[]), // F7
	("SED",			&implied::<SET<{CpuFlags::Decimal}>>()), // F8
	("SBC $nnnn,Y",	&absolute_indexed::<Y, _>(read_op::<SBC>())), // F9
	("*NOP",		&implied::<NOP>()), // FA
	("*ISC",		&[]), // FB
	("*NOP $nnnn,X",&absolute_indexed::<X, _>(read_op::<NOP>())), // FC
	("SBC $nnnn,X",	&absolute_indexed::<X, _>(read_op::<SBC>())), // FD
	("INC $nnnn,X",	&absolute_indexed::<X, _>(rw_op::<INC<BUS, true>>())), // FE
	("*ISC",		&[]), // FF
];
