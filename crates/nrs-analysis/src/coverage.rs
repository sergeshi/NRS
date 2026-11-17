use crate::disasm::{Disassembly, Instruction};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageRange {
    pub start: u16,
    pub end: u16,
}

impl CoverageRange {
    pub fn byte_len(&self) -> usize {
        usize::from(self.end) - usize::from(self.start) + 1
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageReport {
    pub total_bytes: usize,
    pub decoded_bytes: usize,
    pub uncovered_bytes: usize,
    pub coverage_percent: f64,
    pub uncovered_ranges: Vec<CoverageRange>,
}

/// Measures how much of an NROM PRG image is covered by decoded instructions.
///
/// This is byte coverage, not a code/data classifier: uncovered bytes may be
/// data, unreachable code, padding, or bytes hidden behind indirect control flow.
pub fn analyze_coverage(prg_len: usize, disassembly: &Disassembly) -> CoverageReport {
    if prg_len != 0x4000 && prg_len != 0x8000 {
        return CoverageReport {
            total_bytes: prg_len,
            decoded_bytes: 0,
            uncovered_bytes: prg_len,
            coverage_percent: 0.0,
            uncovered_ranges: Vec::new(),
        };
    }

    let mut covered = vec![false; prg_len];

    for instruction in &disassembly.instructions {
        mark_instruction_bytes(&mut covered, prg_len, instruction);
    }

    let decoded_bytes = covered.iter().filter(|&&value| value).count();
    let uncovered_bytes = prg_len - decoded_bytes;
    let coverage_percent = decoded_bytes as f64 * 100.0 / prg_len as f64;

    let mut uncovered_ranges = Vec::new();
    let mut offset = 0;

    while offset < prg_len {
        if covered[offset] {
            offset += 1;
            continue;
        }

        let start = offset;
        while offset + 1 < prg_len && !covered[offset + 1] {
            offset += 1;
        }

        uncovered_ranges.push(CoverageRange {
            start: 0x8000 + start as u16,
            end: 0x8000 + offset as u16,
        });

        offset += 1;
    }

    CoverageReport {
        total_bytes: prg_len,
        decoded_bytes,
        uncovered_bytes,
        coverage_percent,
        uncovered_ranges,
    }
}

fn mark_instruction_bytes(covered: &mut [bool], prg_len: usize, instruction: &Instruction) {
    for index in 0..instruction.bytes.len() {
        let address = instruction.address.wrapping_add(index as u16);
        if address < 0x8000 {
            continue;
        }

        let mut offset = usize::from(address - 0x8000);
        if prg_len == 0x4000 {
            offset &= 0x3FFF;
        }

        if let Some(slot) = covered.get_mut(offset) {
            *slot = true;
        }
    }
}

/// Formats byte coverage and the first uncovered address ranges.
pub fn format_coverage_report(report: &CoverageReport, max_ranges: usize) -> String {
    let mut output = String::from("PRG CODE COVERAGE\n");
    output.push_str("=================\n");
    output.push_str(&format!("Decoded bytes:    {} / {}\n", report.decoded_bytes, report.total_bytes));
    output.push_str(&format!("Uncovered bytes:  {}\n", report.uncovered_bytes));
    output.push_str(&format!("Coverage:         {:.2}%\n", report.coverage_percent));
    output.push_str("Uncovered ranges (possible data, unreachable code, or padding):\n");

    if report.uncovered_ranges.is_empty() {
        output.push_str("  None\n");
    } else {
        for range in report.uncovered_ranges.iter().take(max_ranges) {
            output.push_str(&format!(
                "  ${:04X}-${:04X} ({} bytes)\n",
                range.start,
                range.end,
                range.byte_len()
            ));
        }

        if report.uncovered_ranges.len() > max_ranges {
            output.push_str(&format!(
                "  ... and {} more ranges\n",
                report.uncovered_ranges.len() - max_ranges
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn counts_unique_decoded_bytes_and_reports_uncovered_ranges() {
        let disassembly = Disassembly {
            instructions: vec![instruction(0x8000, &[0xA9, 0x42]), instruction(0x8002, &[0x60])],
            ..Disassembly::default()
        };

        let report = analyze_coverage(0x8000, &disassembly);

        assert_eq!(report.total_bytes, 0x8000);
        assert_eq!(report.decoded_bytes, 3);
        assert_eq!(report.uncovered_bytes, 0x8000 - 3);
        assert_eq!(report.uncovered_ranges[0], CoverageRange { start: 0x8003, end: 0xFFFF });
    }

    #[test]
    fn mirrors_nrom_128_addresses_without_double_counting() {
        let disassembly = Disassembly {
            instructions: vec![instruction(0x8000, &[0xA9, 0x42]), instruction(0xC000, &[0xA9, 0x42])],
            ..Disassembly::default()
        };

        let report = analyze_coverage(0x4000, &disassembly);

        assert_eq!(report.decoded_bytes, 2);
        assert_eq!(report.uncovered_ranges[0].start, 0x8002);
    }

    #[test]
    fn empty_or_unsupported_images_do_not_panic() {
        let empty = analyze_coverage(0, &Disassembly::default());
        assert_eq!(empty.coverage_percent, 0.0);
        assert!(empty.uncovered_ranges.is_empty());

        let unsupported = analyze_coverage(0x2000, &Disassembly::default());
        assert_eq!(unsupported.total_bytes, 0x2000);
        assert_eq!(unsupported.uncovered_bytes, 0x2000);
        assert!(unsupported.uncovered_ranges.is_empty());
    }

    #[test]
    fn report_limits_displayed_ranges() {
        let report = CoverageReport {
            total_bytes: 8,
            decoded_bytes: 4,
            uncovered_bytes: 4,
            coverage_percent: 50.0,
            uncovered_ranges: vec![
                CoverageRange { start: 0x8001, end: 0x8001 },
                CoverageRange { start: 0x8003, end: 0x8003 },
                CoverageRange { start: 0x8005, end: 0x8005 },
                CoverageRange { start: 0x8007, end: 0x8007 },
            ],
        };

        let text = format_coverage_report(&report, 2);
        assert!(text.contains("... and 2 more ranges"));
    }
}
