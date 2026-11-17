use crate::disasm::{Disassembly, Instruction};
use nrs_cpu::Mnemonic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceKind {
    Call,
    Jump,
    Branch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrossReference {
    pub from: u16,
    pub to: u16,
    pub kind: ReferenceKind,
}

impl CrossReference {
    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            ReferenceKind::Call => "CALL",
            ReferenceKind::Jump => "JUMP",
            ReferenceKind::Branch => "BRANCH",
        }
    }
}

fn classify(instruction: &Instruction) -> Option<ReferenceKind> {
    match instruction.mnemonic {
        Mnemonic::JSR => Some(ReferenceKind::Call),
        Mnemonic::JMP => Some(ReferenceKind::Jump),
        _ if instruction.target.is_some() => Some(ReferenceKind::Branch),
        _ => None,
    }
}

/// Builds cross references between statically discovered instructions.
pub fn build_cross_references(disassembly: &Disassembly) -> Vec<CrossReference> {
    let mut references = Vec::new();

    for instruction in &disassembly.instructions {
        let Some(target) = instruction.target else {
            continue;
        };

        let Some(kind) = classify(instruction) else {
            continue;
        };

        references.push(CrossReference {
            from: instruction.address,
            to: target,
            kind,
        });
    }

    references.sort_unstable();
    references.dedup();

    references
}

/// Formats cross references as a readable text report.
pub fn format_cross_references(references: &[CrossReference]) -> String {
    let mut output = String::from("CROSS REFERENCES\n");
    output.push_str("================\n");

    for reference in references {
        output.push_str(&format!(
            "${:04X} -> ${:04X}  {}\n",
            reference.from,
            reference.to,
            reference.kind_name()
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disasm::disassemble_nrom;

    #[test]
    fn collects_calls_jumps_and_branches() {
        let mut prg = vec![0xEA; 0x8000];

        // $8000: JSR $8010
        // $8003: BNE $8007
        // $8005: JMP $8010
        // $8008: RTS
        // $8010: RTS
        prg[0x0000..0x0009]
            .copy_from_slice(&[0x20, 0x10, 0x80, 0xD0, 0x02, 0x4C, 0x10, 0x80, 0x60]);
        prg[0x0010] = 0x60;

        // NMI, RESET, IRQ vectors.
        prg[0x7FFA..0x8000].copy_from_slice(&[0x00, 0x80, 0x00, 0x80, 0x00, 0x80]);

        let disassembly = disassemble_nrom(&prg);
        for instruction in &disassembly.instructions {
            println!(
                "${:04X} {:?} {:?} target={:?}",
                instruction.address, instruction.mnemonic, instruction.mode, instruction.target
            );
        }
        let references = build_cross_references(&disassembly);

        assert!(
            references
                .iter()
                .any(|r| { r.from == 0x8000 && r.to == 0x8010 && r.kind == ReferenceKind::Call })
        );

        assert!(
            references
                .iter()
                .any(|r| { r.from == 0x8003 && r.to == 0x8007 && r.kind == ReferenceKind::Branch })
        );

        assert!(
            references
                .iter()
                .any(|r| { r.from == 0x8005 && r.to == 0x8010 && r.kind == ReferenceKind::Jump })
        );
    }
}
