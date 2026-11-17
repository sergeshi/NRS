use crate::disasm::Disassembly;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteKind {
    Code,
    VectorData,
    Unclassified,
}

impl ByteKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Code => "CODE",
            Self::VectorData => "VECTOR DATA",
            Self::Unclassified => "UNCLASSIFIED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeDataSegment {
    pub start: u16,
    pub end: u16,
    pub kind: ByteKind,
}

impl CodeDataSegment {
    pub fn byte_len(&self) -> usize {
        usize::from(self.end) - usize::from(self.start) + 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CodeDataReport {
    pub total_bytes: usize,
    pub code_bytes: usize,
    pub vector_data_bytes: usize,
    pub unclassified_bytes: usize,
    pub segments: Vec<CodeDataSegment>,
}

/// Conservatively classifies statically decoded instruction bytes and the
/// six-byte NROM interrupt-vector table. Other bytes remain unclassified:
/// they may contain data, unreachable code, padding, or indirect code targets.
pub fn analyze_code_data(prg_len: usize, disassembly: &Disassembly) -> CodeDataReport {
    if prg_len != 0x4000 && prg_len != 0x8000 {
        return CodeDataReport {
            total_bytes: prg_len,
            unclassified_bytes: prg_len,
            ..CodeDataReport::default()
        };
    }

    let mut kinds = vec![ByteKind::Unclassified; prg_len];

    // The final six physical PRG bytes hold NMI, RESET, and IRQ/BRK vectors.
    for kind in kinds.iter_mut().skip(prg_len - 6) {
        *kind = ByteKind::VectorData;
    }

    for instruction in &disassembly.instructions {
        for index in 0..instruction.bytes.len() {
            let address = instruction.address.wrapping_add(index as u16);
            if address < 0x8000 {
                continue;
            }

            let mut offset = usize::from(address - 0x8000);
            if prg_len == 0x4000 {
                offset &= 0x3FFF;
            }

            if let Some(kind) = kinds.get_mut(offset) {
                // Vector bytes are known metadata, even if traversal reaches them.
                if *kind != ByteKind::VectorData {
                    *kind = ByteKind::Code;
                }
            }
        }
    }

    let code_bytes = kinds.iter().filter(|&&kind| kind == ByteKind::Code).count();
    let vector_data_bytes = kinds
        .iter()
        .filter(|&&kind| kind == ByteKind::VectorData)
        .count();
    let unclassified_bytes = prg_len - code_bytes - vector_data_bytes;

    let mut segments = Vec::new();
    let mut start = 0;
    while start < kinds.len() {
        let kind = kinds[start];
        let mut end = start;
        while end + 1 < kinds.len() && kinds[end + 1] == kind {
            end += 1;
        }

        segments.push(CodeDataSegment {
            start: 0x8000 + start as u16,
            end: 0x8000 + end as u16,
            kind,
        });
        start = end + 1;
    }

    CodeDataReport {
        total_bytes: prg_len,
        code_bytes,
        vector_data_bytes,
        unclassified_bytes,
        segments,
    }
}

pub fn format_code_data_report(report: &CodeDataReport, max_segments: usize) -> String {
    let mut output = String::from("CODE / DATA CLASSIFICATION\n");
    output.push_str("==========================\n");
    output.push_str(&format!("Code bytes:        {}\n", report.code_bytes));
    output.push_str(&format!("Vector data bytes: {}\n", report.vector_data_bytes));
    output.push_str(&format!("Unclassified:      {}\n", report.unclassified_bytes));
    output.push_str("Segments (physical PRG address space):\n");

    for segment in report.segments.iter().take(max_segments) {
        output.push_str(&format!(
            "  ${:04X}-${:04X}  {:<13} {} bytes\n",
            segment.start,
            segment.end,
            segment.kind.name(),
            segment.byte_len()
        ));
    }

    if report.segments.len() > max_segments {
        output.push_str(&format!(
            "  ... and {} more segments\n",
            report.segments.len() - max_segments
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disasm::Instruction;
    use nrs_cpu::{AddressingMode, Mnemonic};

    fn instruction(address: u16, bytes: &[u8]) -> Instruction {
        Instruction {
            address,
            bytes: bytes.to_vec(),
            mnemonic: Mnemonic::LDA,
            mode: AddressingMode::Immediate,
            target: None,
            unofficial: false,
        }
    }

    #[test]
    fn separates_decoded_code_vectors_and_unknown_bytes() {
        let disassembly = Disassembly {
            instructions: vec![instruction(0x8000, &[0xA9, 0x42, 0x60])],
            ..Disassembly::default()
        };

        let report = analyze_code_data(0x8000, &disassembly);
        assert_eq!(report.code_bytes, 3);
        assert_eq!(report.vector_data_bytes, 6);
        assert_eq!(report.unclassified_bytes, 0x8000 - 9);
        assert_eq!(report.segments.last().unwrap().kind, ByteKind::VectorData);
    }

    #[test]
    fn mirrors_nrom_128_code_addresses() {
        let disassembly = Disassembly {
            instructions: vec![instruction(0xC000, &[0xA9, 0x42])],
            ..Disassembly::default()
        };

        let report = analyze_code_data(0x4000, &disassembly);
        assert_eq!(report.code_bytes, 2);
    }

    #[test]
    fn vector_bytes_are_not_mislabeled_as_code() {
        let disassembly = Disassembly {
            instructions: vec![instruction(0xFFFA, &[0xA9, 0x42])],
            ..Disassembly::default()
        };

        let report = analyze_code_data(0x8000, &disassembly);
        assert_eq!(report.code_bytes, 0);
        assert_eq!(report.vector_data_bytes, 6);
    }

    #[test]
    fn unsupported_prg_size_is_left_unclassified() {
        let report = analyze_code_data(0x2000, &Disassembly::default());
        assert_eq!(report.unclassified_bytes, 0x2000);
        assert!(report.segments.is_empty());
    }
}
