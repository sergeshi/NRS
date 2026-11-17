use crate::disasm::{Disassembly, Instruction};
use nrs_cpu::{AddressingMode, Mnemonic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryRegion {
    ZeroPageRam,
    InternalRam,
    PpuRegisters,
    ApuIo,
    Expansion,
    SaveRam,
    PrgRom,
}

impl MemoryRegion {
    pub fn name(self) -> &'static str {
        match self {
            Self::ZeroPageRam => "ZERO PAGE RAM",
            Self::InternalRam => "INTERNAL RAM",
            Self::PpuRegisters => "PPU REGISTERS",
            Self::ApuIo => "APU / I-O",
            Self::Expansion => "EXPANSION",
            Self::SaveRam => "SAVE RAM",
            Self::PrgRom => "PRG ROM",
        }
    }
}

pub fn classify_memory_address(address: u16) -> MemoryRegion {
    match address {
        0x0000..=0x00FF => MemoryRegion::ZeroPageRam,
        0x0100..=0x1FFF => MemoryRegion::InternalRam,
        0x2000..=0x3FFF => MemoryRegion::PpuRegisters,
        0x4000..=0x4017 => MemoryRegion::ApuIo,
        0x4018..=0x5FFF => MemoryRegion::Expansion,
        0x6000..=0x7FFF => MemoryRegion::SaveRam,
        0x8000..=0xFFFF => MemoryRegion::PrgRom,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryAccess {
    Read,
    Write,
    ReadWrite,
    PointerRead,
}

impl MemoryAccess {
    pub fn name(self) -> &'static str {
        match self {
            Self::Read => "READ",
            Self::Write => "WRITE",
            Self::ReadWrite => "READ/WRITE",
            Self::PointerRead => "POINTER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryReference {
    pub instruction_address: u16,
    /// Encoded base address or zero-page pointer slot, not always the final effective address.
    pub address: u16,
    pub region: MemoryRegion,
    pub access: MemoryAccess,
    pub indexed: bool,
    pub indirect: bool,
    pub effective_address_known: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryReferenceReport {
    pub references: Vec<MemoryReference>,
    pub direct_references: usize,
    pub indexed_references: usize,
    pub indirect_references: usize,
}

fn memory_access(instruction: &Instruction) -> MemoryAccess {
    use Mnemonic::*;
    match instruction.mnemonic {
        STA | STX | STY | SAX | AHX | SHA | SHX | SHY | TAS => MemoryAccess::Write,
        ASL | LSR | ROL | ROR | INC | DEC | SLO | RLA | SRE | RRA | DCP | ISC => {
            MemoryAccess::ReadWrite
        }
        _ => MemoryAccess::Read,
    }
}

fn operand_address(instruction: &Instruction) -> Option<u16> {
    let low = *instruction.bytes.get(1)?;
    let high = *instruction.bytes.get(2).unwrap_or(&0);
    Some(u16::from_le_bytes([low, high]))
}

fn reference_for(instruction: &Instruction) -> Option<MemoryReference> {
    let address = operand_address(instruction)?;

    let (address, access, indexed, indirect, known) = match instruction.mode {
        AddressingMode::ZeroPage => {
            (address & 0x00FF, memory_access(instruction), false, false, true)
        }
        AddressingMode::ZeroPageX | AddressingMode::ZeroPageY => {
            (address & 0x00FF, memory_access(instruction), true, false, false)
        }
        AddressingMode::Absolute | AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => {
            if matches!(instruction.mnemonic, Mnemonic::JMP | Mnemonic::JSR) {
                return None;
            }
            let indexed = matches!(
                instruction.mode,
                AddressingMode::AbsoluteX | AddressingMode::AbsoluteY
            );
            (address, memory_access(instruction), indexed, false, !indexed)
        }
        AddressingMode::Indirect => {
            if instruction.mnemonic != Mnemonic::JMP {
                return None;
            }
            (address, MemoryAccess::PointerRead, false, true, false)
        }
        AddressingMode::IndexedIndirect | AddressingMode::IndirectIndexed => {
            // The encoded zero-page slot is read as a pointer; the instruction's
            // data access applies to an effective address that static analysis cannot resolve.
            (address & 0x00FF, memory_access(instruction), true, true, false)
        }
        AddressingMode::Implied
        | AddressingMode::Accumulator
        | AddressingMode::Immediate
        | AddressingMode::Relative => return None,
    };

    Some(MemoryReference {
        instruction_address: instruction.address,
        address,
        region: classify_memory_address(address),
        access,
        indexed,
        indirect,
        effective_address_known: known,
    })
}

/// Extracts statically visible memory operands. Indexed operands retain their
/// encoded base; indirect operands identify pointer storage, not a guessed target.
pub fn analyze_memory_references(disassembly: &Disassembly) -> MemoryReferenceReport {
    let mut report = MemoryReferenceReport::default();

    for instruction in &disassembly.instructions {
        let Some(reference) = reference_for(instruction) else {
            continue;
        };

        if reference.indirect {
            report.indirect_references += 1;
        } else if reference.indexed {
            report.indexed_references += 1;
        } else {
            report.direct_references += 1;
        }
        report.references.push(reference);
    }

    report.references
        .sort_by_key(|reference| reference.instruction_address);
    report
}

pub fn format_memory_reference_report(
    report: &MemoryReferenceReport,
    max_references: usize,
) -> String {
    let mut output = String::from("MEMORY REFERENCES\n");
    output.push_str("=================\n");
    output.push_str(&format!("Total references:   {}\n", report.references.len()));
    output.push_str(&format!("Direct:             {}\n", report.direct_references));
    output.push_str(&format!("Indexed:            {}\n", report.indexed_references));
    output.push_str(&format!("Indirect:           {}\n", report.indirect_references));
    output.push_str("References (encoded base / pointer address):\n");

    for reference in report.references.iter().take(max_references) {
        let qualifier = if reference.indirect {
            " indirect"
        } else if reference.indexed {
            " indexed"
        } else {
            ""
        };
        let certainty = if reference.effective_address_known {
            ""
        } else {
            " [effective address unresolved]"
        };
        output.push_str(&format!(
            "  ${:04X} -> ${:04X}  {:<10} {:<14}{}{}\n",
            reference.instruction_address,
            reference.address,
            reference.access.name(),
            reference.region.name(),
            qualifier,
            certainty
        ));
    }

    if report.references.len() > max_references {
        output.push_str(&format!(
            "  ... and {} more references\n",
            report.references.len() - max_references
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disasm::Disassembly;

    fn instruction(
        address: u16,
        bytes: &[u8],
        mnemonic: Mnemonic,
        mode: AddressingMode,
    ) -> Instruction {
        Instruction {
            address,
            bytes: bytes.to_vec(),
            mnemonic,
            mode,
            target: None,
            unofficial: false,
        }
    }

    #[test]
    fn classifies_nes_memory_map_regions() {
        assert_eq!(classify_memory_address(0x0010), MemoryRegion::ZeroPageRam);
        assert_eq!(classify_memory_address(0x0800), MemoryRegion::InternalRam);
        assert_eq!(classify_memory_address(0x2008), MemoryRegion::PpuRegisters);
        assert_eq!(classify_memory_address(0x4016), MemoryRegion::ApuIo);
        assert_eq!(classify_memory_address(0x5000), MemoryRegion::Expansion);
        assert_eq!(classify_memory_address(0x6000), MemoryRegion::SaveRam);
        assert_eq!(classify_memory_address(0x8000), MemoryRegion::PrgRom);
    }

    #[test]
    fn detects_direct_reads_writes_and_read_modify_write() {
        let disassembly = Disassembly {
            instructions: vec![
                instruction(0x8000, &[0xAD, 0x02, 0x20], Mnemonic::LDA, AddressingMode::Absolute),
                instruction(0x8003, &[0x8D, 0x00, 0x20], Mnemonic::STA, AddressingMode::Absolute),
                instruction(0x8006, &[0xEE, 0x10, 0x00], Mnemonic::INC, AddressingMode::Absolute),
            ],
            ..Disassembly::default()
        };

        let report = analyze_memory_references(&disassembly);
        assert_eq!(report.references.len(), 3);
        assert_eq!(report.references[0].access, MemoryAccess::Read);
        assert_eq!(report.references[1].access, MemoryAccess::Write);
        assert_eq!(report.references[2].access, MemoryAccess::ReadWrite);
        assert_eq!(report.references[0].region, MemoryRegion::PpuRegisters);
    }

    #[test]
    fn marks_indexed_and_indirect_addresses_as_unresolved() {
        let disassembly = Disassembly {
            instructions: vec![
                instruction(0x8000, &[0xBD, 0x00, 0x20], Mnemonic::LDA, AddressingMode::AbsoluteX),
                instruction(0x8003, &[0xB1, 0x20], Mnemonic::LDA, AddressingMode::IndirectIndexed),
                instruction(0x8005, &[0x6C, 0x34, 0x12], Mnemonic::JMP, AddressingMode::Indirect),
            ],
            ..Disassembly::default()
        };

        let report = analyze_memory_references(&disassembly);
        assert_eq!(report.references.len(), 3);
        assert!(report.references.iter().all(|reference| !reference.effective_address_known));
        assert_eq!(report.indirect_references, 2);
        assert_eq!(report.indexed_references, 1);
    }

    #[test]
    fn does_not_treat_control_flow_targets_as_memory_operands() {
        let disassembly = Disassembly {
            instructions: vec![
                instruction(0x8000, &[0x20, 0x00, 0x90], Mnemonic::JSR, AddressingMode::Absolute),
                instruction(0x8003, &[0x4C, 0x00, 0x90], Mnemonic::JMP, AddressingMode::Absolute),
            ],
            ..Disassembly::default()
        };
        assert!(analyze_memory_references(&disassembly).references.is_empty());
    }

    #[test]
    fn excludes_immediate_and_implied_operands() {
        let disassembly = Disassembly {
            instructions: vec![
                instruction(0x8000, &[0xA9, 0x20], Mnemonic::LDA, AddressingMode::Immediate),
                instruction(0x8002, &[0xEA], Mnemonic::NOP, AddressingMode::Implied),
            ],
            ..Disassembly::default()
        };
        assert!(analyze_memory_references(&disassembly).references.is_empty());
    }
}
