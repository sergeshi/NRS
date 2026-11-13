#![forbid(unsafe_code)]

//! Opcode metadata and decoder for the Ricoh 2A03 (NES CPU).
//! The table covers all 256 byte values, including stable and unstable
//! unofficial opcodes. Cycle counts are base instruction timings; conditional
//! page-cross and branch penalties are exposed separately.

pub mod cycle;
pub use cycle::{BusAccess, CpuFault, CycleCpu, CycleCpuState};

/// Processor status register bit masks.
pub mod flags {
    pub const CARRY: u8 = 0x01;
    pub const ZERO: u8 = 0x02;
    pub const INTERRUPT_DISABLE: u8 = 0x04;
    pub const DECIMAL: u8 = 0x08;
    pub const BREAK: u8 = 0x10;
    pub const UNUSED: u8 = 0x20;
    pub const OVERFLOW: u8 = 0x40;
    pub const NEGATIVE: u8 = 0x80;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressingMode {
    Implied, Accumulator, Immediate, ZeroPage, ZeroPageX, ZeroPageY,
    Absolute, AbsoluteX, AbsoluteY, Indirect, IndexedIndirect,
    IndirectIndexed, Relative,
}

impl AddressingMode {
    pub const fn instruction_bytes(self) -> u8 {
        match self {
            Self::Implied | Self::Accumulator => 1,
            Self::Immediate | Self::ZeroPage | Self::ZeroPageX | Self::ZeroPageY
            | Self::IndexedIndirect | Self::IndirectIndexed | Self::Relative => 2,
            Self::Absolute | Self::AbsoluteX | Self::AbsoluteY | Self::Indirect => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mnemonic {
    ADC, AHX, ALR, ANC, AND, ARR, ASL, AXS, BCC, BCS, BEQ, BIT, BMI, BNE,
    BPL, BRK, BVC, BVS, CLC, CLD, CLI, CLV, CMP, CPX, CPY, DCP, DEC, DEX,
    DEY, EOR, INC, INX, INY, ISC, JMP, JSR, KIL, LAS, LAX, LDA, LDX, LDY,
    LSR, NOP, ORA, PHA, PHP, PLA, PLP, RLA, ROL, ROR, RRA, RTI, RTS, SAX,
    SBC, SEC, SED, SEI, SHA, SHX, SHY, SLO, SRE, STA, STX, STY, TAS, TAX,
    TAY, TSX, TXA, TXS, TYA, XAA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opcode {
    pub code: u8,
    pub mnemonic: Mnemonic,
    pub mode: AddressingMode,
    pub bytes: u8,
    pub base_cycles: u8,
    pub unofficial: bool,
    pub page_cross_penalty: bool,
    pub is_branch: bool,
    pub jams_cpu: bool,
}

macro_rules! op {
    ($code:expr, $mnemonic:ident, $mode:ident, $cycles:expr, $unofficial:expr) => {
        Opcode {
            code: $code,
            mnemonic: Mnemonic::$mnemonic,
            mode: AddressingMode::$mode,
            bytes: AddressingMode::$mode.instruction_bytes(),
            base_cycles: $cycles,
            unofficial: $unofficial,
            page_cross_penalty: matches!(AddressingMode::$mode,
                AddressingMode::AbsoluteX | AddressingMode::AbsoluteY | AddressingMode::IndirectIndexed)
                && matches!(Mnemonic::$mnemonic,
                    Mnemonic::ADC | Mnemonic::AND | Mnemonic::CMP | Mnemonic::EOR |
                    Mnemonic::LDA | Mnemonic::LDX | Mnemonic::LDY | Mnemonic::ORA |
                    Mnemonic::SBC | Mnemonic::LAX | Mnemonic::LAS | Mnemonic::NOP),
            is_branch: matches!(Mnemonic::$mnemonic,
                Mnemonic::BCC | Mnemonic::BCS | Mnemonic::BEQ | Mnemonic::BMI |
                Mnemonic::BNE | Mnemonic::BPL | Mnemonic::BVC | Mnemonic::BVS),
            jams_cpu: matches!(Mnemonic::$mnemonic, Mnemonic::KIL),
        }
    };
}

/// Canonical 256-entry Ricoh 2A03 opcode matrix, indexed directly by opcode.
pub const OPCODES: [Opcode; 256] = [
    op!(0x00, BRK, Imp, 7, false),
    op!(0x01, ORA, IndX, 6, false),
    op!(0x02, KIL, Imp, 2, true),
    op!(0x03, SLO, IndX, 8, true),
    op!(0x04, NOP, Zp, 3, true),
    op!(0x05, ORA, Zp, 3, false),
    op!(0x06, ASL, Zp, 5, false),
    op!(0x07, SLO, Zp, 5, true),
    op!(0x08, PHP, Imp, 3, false),
    op!(0x09, ORA, Imm, 2, false),
    op!(0x0a, ASL, Acc, 2, false),
    op!(0x0b, ANC, Imm, 2, true),
    op!(0x0c, NOP, Abs, 4, true),
    op!(0x0d, ORA, Abs, 4, false),
    op!(0x0e, ASL, Abs, 6, false),
    op!(0x0f, SLO, Abs, 6, true),
    op!(0x10, BPL, Rel, 2, false),
    op!(0x11, ORA, IndY, 5, false),
    op!(0x12, KIL, Imp, 2, true),
    op!(0x13, SLO, IndY, 8, true),
    op!(0x14, NOP, ZpX, 4, true),
    op!(0x15, ORA, ZpX, 4, false),
    op!(0x16, ASL, ZpX, 6, false),
    op!(0x17, SLO, ZpX, 6, true),
    op!(0x18, CLC, Imp, 2, false),
    op!(0x19, ORA, AbsY, 4, false),
    op!(0x1a, NOP, Imp, 2, true),
    op!(0x1b, SLO, AbsY, 7, true),
    op!(0x1c, NOP, AbsX, 4, true),
    op!(0x1d, ORA, AbsX, 4, false),
    op!(0x1e, ASL, AbsX, 7, false),
    op!(0x1f, SLO, AbsX, 7, true),
    op!(0x20, JSR, Abs, 6, false),
    op!(0x21, AND, IndX, 6, false),
    op!(0x22, KIL, Imp, 2, true),
    op!(0x23, RLA, IndX, 8, true),
    op!(0x24, BIT, Zp, 3, false),
    op!(0x25, AND, Zp, 3, false),
    op!(0x26, ROL, Zp, 5, false),
    op!(0x27, RLA, Zp, 5, true),
    op!(0x28, PLP, Imp, 4, false),
    op!(0x29, AND, Imm, 2, false),
    op!(0x2a, ROL, Acc, 2, false),
    op!(0x2b, ANC, Imm, 2, true),
    op!(0x2c, BIT, Abs, 4, false),
    op!(0x2d, AND, Abs, 4, false),
    op!(0x2e, ROL, Abs, 6, false),
    op!(0x2f, RLA, Abs, 6, true),
    op!(0x30, BMI, Rel, 2, false),
    op!(0x31, AND, IndY, 5, false),
    op!(0x32, KIL, Imp, 2, true),
    op!(0x33, RLA, IndY, 8, true),
    op!(0x34, NOP, ZpX, 4, true),
    op!(0x35, AND, ZpX, 4, false),
    op!(0x36, ROL, ZpX, 6, false),
    op!(0x37, RLA, ZpX, 6, true),
    op!(0x38, SEC, Imp, 2, false),
    op!(0x39, AND, AbsY, 4, false),
    op!(0x3a, NOP, Imp, 2, true),
    op!(0x3b, RLA, AbsY, 7, true),
    op!(0x3c, NOP, AbsX, 4, true),
    op!(0x3d, AND, AbsX, 4, false),
    op!(0x3e, ROL, AbsX, 7, false),
    op!(0x3f, RLA, AbsX, 7, true),
    op!(0x40, RTI, Imp, 6, false),
    op!(0x41, EOR, IndX, 6, false),
    op!(0x42, KIL, Imp, 2, true),
    op!(0x43, SRE, IndX, 8, true),
    op!(0x44, NOP, Zp, 3, true),
    op!(0x45, EOR, Zp, 3, false),
    op!(0x46, LSR, Zp, 5, false),
    op!(0x47, SRE, Zp, 5, true),
    op!(0x48, PHA, Imp, 3, false),
    op!(0x49, EOR, Imm, 2, false),
    op!(0x4a, LSR, Acc, 2, false),
    op!(0x4b, ALR, Imm, 2, true),
    op!(0x4c, JMP, Abs, 3, false),
    op!(0x4d, EOR, Abs, 4, false),
    op!(0x4e, LSR, Abs, 6, false),
    op!(0x4f, SRE, Abs, 6, true),
    op!(0x50, BVC, Rel, 2, false),
    op!(0x51, EOR, IndY, 5, false),
    op!(0x52, KIL, Imp, 2, true),
    op!(0x53, SRE, IndY, 8, true),
    op!(0x54, NOP, ZpX, 4, true),
    op!(0x55, EOR, ZpX, 4, false),
    op!(0x56, LSR, ZpX, 6, false),
    op!(0x57, SRE, ZpX, 6, true),
    op!(0x58, CLI, Imp, 2, false),
    op!(0x59, EOR, AbsY, 4, false),
    op!(0x5a, NOP, Imp, 2, true),
    op!(0x5b, SRE, AbsY, 7, true),
    op!(0x5c, NOP, AbsX, 4, true),
    op!(0x5d, EOR, AbsX, 4, false),
    op!(0x5e, LSR, AbsX, 7, false),
    op!(0x5f, SRE, AbsX, 7, true),
    op!(0x60, RTS, Imp, 6, false),
    op!(0x61, ADC, IndX, 6, false),
    op!(0x62, KIL, Imp, 2, true),
    op!(0x63, RRA, IndX, 8, true),
    op!(0x64, NOP, Zp, 3, true),
    op!(0x65, ADC, Zp, 3, false),
    op!(0x66, ROR, Zp, 5, false),
    op!(0x67, RRA, Zp, 5, true),
    op!(0x68, PLA, Imp, 4, false),
    op!(0x69, ADC, Imm, 2, false),
    op!(0x6a, ROR, Acc, 2, false),
    op!(0x6b, ARR, Imm, 2, true),
    op!(0x6c, JMP, Ind, 5, false),
    op!(0x6d, ADC, Abs, 4, false),
    op!(0x6e, ROR, Abs, 6, false),
    op!(0x6f, RRA, Abs, 6, true),
    op!(0x70, BVS, Rel, 2, false),
    op!(0x71, ADC, IndY, 5, false),
    op!(0x72, KIL, Imp, 2, true),
    op!(0x73, RRA, IndY, 8, true),
    op!(0x74, NOP, ZpX, 4, true),
    op!(0x75, ADC, ZpX, 4, false),
    op!(0x76, ROR, ZpX, 6, false),
    op!(0x77, RRA, ZpX, 6, true),
    op!(0x78, SEI, Imp, 2, false),
    op!(0x79, ADC, AbsY, 4, false),
    op!(0x7a, NOP, Imp, 2, true),
    op!(0x7b, RRA, AbsY, 7, true),
    op!(0x7c, NOP, AbsX, 4, true),
    op!(0x7d, ADC, AbsX, 4, false),
    op!(0x7e, ROR, AbsX, 7, false),
    op!(0x7f, RRA, AbsX, 7, true),
    op!(0x80, NOP, Imm, 2, true),
    op!(0x81, STA, IndX, 6, false),
    op!(0x82, NOP, Imm, 2, true),
    op!(0x83, SAX, IndX, 6, true),
    op!(0x84, STY, Zp, 3, false),
    op!(0x85, STA, Zp, 3, false),
    op!(0x86, STX, Zp, 3, false),
    op!(0x87, SAX, Zp, 3, true),
    op!(0x88, DEY, Imp, 2, false),
    op!(0x89, NOP, Imm, 2, true),
    op!(0x8a, TXA, Imp, 2, false),
    op!(0x8b, XAA, Imm, 2, true),
    op!(0x8c, STY, Abs, 4, false),
    op!(0x8d, STA, Abs, 4, false),
    op!(0x8e, STX, Abs, 4, false),
    op!(0x8f, SAX, Abs, 4, true),
    op!(0x90, BCC, Rel, 2, false),
    op!(0x91, STA, IndY, 6, false),
    op!(0x92, KIL, Imp, 2, true),
    op!(0x93, AHX, IndY, 6, true),
    op!(0x94, STY, ZpX, 4, false),
    op!(0x95, STA, ZpX, 4, false),
    op!(0x96, STX, ZpY, 4, false),
    op!(0x97, SAX, ZpY, 4, true),
    op!(0x98, TYA, Imp, 2, false),
    op!(0x99, STA, AbsY, 5, false),
    op!(0x9a, TXS, Imp, 2, false),
    op!(0x9b, TAS, AbsY, 5, true),
    op!(0x9c, SHY, AbsX, 5, true),
    op!(0x9d, STA, AbsX, 5, false),
    op!(0x9e, SHX, AbsY, 5, true),
    op!(0x9f, AHX, AbsY, 5, true),
    op!(0xa0, LDY, Imm, 2, false),
    op!(0xa1, LDA, IndX, 6, false),
    op!(0xa2, LDX, Imm, 2, false),
    op!(0xa3, LAX, IndX, 6, true),
    op!(0xa4, LDY, Zp, 3, false),
    op!(0xa5, LDA, Zp, 3, false),
    op!(0xa6, LDX, Zp, 3, false),
    op!(0xa7, LAX, Zp, 3, true),
    op!(0xa8, TAY, Imp, 2, false),
    op!(0xa9, LDA, Imm, 2, false),
    op!(0xaa, TAX, Imp, 2, false),
    op!(0xab, LAX, Imm, 2, true),
    op!(0xac, LDY, Abs, 4, false),
    op!(0xad, LDA, Abs, 4, false),
    op!(0xae, LDX, Abs, 4, false),
    op!(0xaf, LAX, Abs, 4, true),
    op!(0xb0, BCS, Rel, 2, false),
    op!(0xb1, LDA, IndY, 5, false),
    op!(0xb2, KIL, Imp, 2, true),
    op!(0xb3, LAX, IndY, 5, true),
    op!(0xb4, LDY, ZpX, 4, false),
    op!(0xb5, LDA, ZpX, 4, false),
    op!(0xb6, LDX, ZpY, 4, false),
    op!(0xb7, LAX, ZpY, 4, true),
    op!(0xb8, CLV, Imp, 2, false),
    op!(0xb9, LDA, AbsY, 4, false),
    op!(0xba, TSX, Imp, 2, false),
    op!(0xbb, LAS, AbsY, 4, true),
    op!(0xbc, LDY, AbsX, 4, false),
    op!(0xbd, LDA, AbsX, 4, false),
    op!(0xbe, LDX, AbsY, 4, false),
    op!(0xbf, LAX, AbsY, 4, true),
    op!(0xc0, CPY, Imm, 2, false),
    op!(0xc1, CMP, IndX, 6, false),
    op!(0xc2, NOP, Imm, 2, true),
    op!(0xc3, DCP, IndX, 8, true),
    op!(0xc4, CPY, Zp, 3, false),
    op!(0xc5, CMP, Zp, 3, false),
    op!(0xc6, DEC, Zp, 5, false),
    op!(0xc7, DCP, Zp, 5, true),
    op!(0xc8, INY, Imp, 2, false),
    op!(0xc9, CMP, Imm, 2, false),
    op!(0xca, DEX, Imp, 2, false),
    op!(0xcb, AXS, Imm, 2, true),
    op!(0xcc, CPY, Abs, 4, false),
    op!(0xcd, CMP, Abs, 4, false),
    op!(0xce, DEC, Abs, 6, false),
    op!(0xcf, DCP, Abs, 6, true),
    op!(0xd0, BNE, Rel, 2, false),
    op!(0xd1, CMP, IndY, 5, false),
    op!(0xd2, KIL, Imp, 2, true),
    op!(0xd3, DCP, IndY, 8, true),
    op!(0xd4, NOP, ZpX, 4, true),
    op!(0xd5, CMP, ZpX, 4, false),
    op!(0xd6, DEC, ZpX, 6, false),
    op!(0xd7, DCP, ZpX, 6, true),
    op!(0xd8, CLD, Imp, 2, false),
    op!(0xd9, CMP, AbsY, 4, false),
    op!(0xda, NOP, Imp, 2, true),
    op!(0xdb, DCP, AbsY, 7, true),
    op!(0xdc, NOP, AbsX, 4, true),
    op!(0xdd, CMP, AbsX, 4, false),
    op!(0xde, DEC, AbsX, 7, false),
    op!(0xdf, DCP, AbsX, 7, true),
    op!(0xe0, CPX, Imm, 2, false),
    op!(0xe1, SBC, IndX, 6, false),
    op!(0xe2, NOP, Imm, 2, true),
    op!(0xe3, ISC, IndX, 8, true),
    op!(0xe4, CPX, Zp, 3, false),
    op!(0xe5, SBC, Zp, 3, false),
    op!(0xe6, INC, Zp, 5, false),
    op!(0xe7, ISC, Zp, 5, true),
    op!(0xe8, INX, Imp, 2, false),
    op!(0xe9, SBC, Imm, 2, false),
    op!(0xea, NOP, Imp, 2, false),
    op!(0xeb, SBC, Imm, 2, true),
    op!(0xec, CPX, Abs, 4, false),
    op!(0xed, SBC, Abs, 4, false),
    op!(0xee, INC, Abs, 6, false),
    op!(0xef, ISC, Abs, 6, true),
    op!(0xf0, BEQ, Rel, 2, false),
    op!(0xf1, SBC, IndY, 5, false),
    op!(0xf2, KIL, Imp, 2, true),
    op!(0xf3, ISC, IndY, 8, true),
    op!(0xf4, NOP, ZpX, 4, true),
    op!(0xf5, SBC, ZpX, 4, false),
    op!(0xf6, INC, ZpX, 6, false),
    op!(0xf7, ISC, ZpX, 6, true),
    op!(0xf8, SED, Imp, 2, false),
    op!(0xf9, SBC, AbsY, 4, false),
    op!(0xfa, NOP, Imp, 2, true),
    op!(0xfb, ISC, AbsY, 7, true),
    op!(0xfc, NOP, AbsX, 4, true),
    op!(0xfd, SBC, AbsX, 4, false),
    op!(0xfe, INC, AbsX, 7, false),
    op!(0xff, ISC, AbsX, 7, true),
];

/// Decodes any opcode byte. Every possible byte has a defined entry.
pub const fn decode(code: u8) -> &'static Opcode {
    &OPCODES[code as usize]
}

/// Returns the complete opcode metadata table.
pub const fn opcode_table() -> &'static [Opcode; 256] { &OPCODES }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuState {
    pub a: u8, pub x: u8, pub y: u8, pub stack_pointer: u8,
    pub program_counter: u16, pub status: u8, pub opcode: u8,
    pub micro_cycle: u8, pub total_cycles: u64, pub nmi_pending: bool,
    pub irq_line: bool,
}

impl Default for CpuState {
    fn default() -> Self {
        Self { a: 0, x: 0, y: 0, stack_pointer: 0xFD, program_counter: 0,
            status: flags::INTERRUPT_DISABLE | flags::UNUSED, opcode: 0,
            micro_cycle: 0, total_cycles: 0, nmi_pending: false, irq_line: false }
    }
}

/// External CPU bus. The cycle engine will issue exactly one read or write
/// per clock tick through this interface.
pub trait CpuBus {
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cpu { state: CpuState }

impl Cpu {
    pub const fn new() -> Self {
        Self { state: CpuState { a: 0, x: 0, y: 0, stack_pointer: 0xFD,
            program_counter: 0, status: flags::INTERRUPT_DISABLE | flags::UNUSED,
            opcode: 0, micro_cycle: 0, total_cycles: 0, nmi_pending: false, irq_line: false } }
    }
    pub const fn state(&self) -> CpuState { self.state }
    pub fn restore_state(&mut self, mut state: CpuState) {
        state.status |= flags::UNUSED;
        self.state = state;
    }
    pub fn set_register_a(&mut self, value: u8) { self.state.a = value; }
    pub fn set_register_x(&mut self, value: u8) { self.state.x = value; }
    pub fn set_register_y(&mut self, value: u8) { self.state.y = value; }
    pub fn set_program_counter(&mut self, value: u16) { self.state.program_counter = value; }
    pub const fn flag(&self, mask: u8) -> bool { self.state.status & mask != 0 }
    pub fn set_flag(&mut self, mask: u8, enabled: bool) {
        if enabled { self.state.status |= mask; } else { self.state.status &= !mask; }
        self.state.status |= flags::UNUSED;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_possible_opcode_decodes_to_its_own_table_slot() {
        for value in 0u16..=255 {
            let code = value as u8;
            let decoded = decode(code);
            assert_eq!(decoded.code, code);
            assert_eq!(decoded, &OPCODES[value as usize]);
            assert!((1..=3).contains(&decoded.bytes));
            assert!(decoded.base_cycles >= 2);
        }
    }

    #[test]
    fn opcode_table_has_exactly_256_unique_entries() {
        assert_eq!(OPCODES.len(), 256);
        for (index, opcode) in OPCODES.iter().enumerate() {
            assert_eq!(opcode.code as usize, index);
        }
    }

    #[test]
    fn official_and_unofficial_opcodes_are_both_represented() {
        assert!(!decode(0xA9).unofficial); // LDA immediate
        assert!(!decode(0x00).unofficial); // BRK
        assert!(decode(0x03).unofficial);  // SLO indexed-indirect
        assert!(decode(0xEB).unofficial);  // unofficial SBC immediate
        assert!(decode(0x02).jams_cpu);    // KIL
    }

    #[test]
    fn addressing_mode_lengths_are_consistent() {
        for opcode in OPCODES {
            assert_eq!(opcode.bytes, opcode.mode.instruction_bytes());
        }
    }

    #[test]
    fn branch_and_page_cross_metadata_is_classified() {
        assert!(decode(0xD0).is_branch);
        assert!(!decode(0xD0).page_cross_penalty);
        assert!(decode(0xBD).page_cross_penalty);
        assert!(!decode(0x9D).page_cross_penalty); // STA abs,X is fixed-cycle
    }

    #[test]
    fn cpu_state_and_flags_remain_snapshot_friendly() {
        let mut cpu = Cpu::new();
        cpu.set_flag(flags::CARRY, true);
        assert!(cpu.flag(flags::CARRY));
        let mut snapshot = cpu.state();
        snapshot.program_counter = 0x8123;
        cpu.restore_state(snapshot);
        assert_eq!(cpu.state().program_counter, 0x8123);
        assert!(cpu.flag(flags::UNUSED));
    }
}
