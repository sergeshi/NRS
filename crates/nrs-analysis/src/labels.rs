use std::collections::{BTreeMap, BTreeSet};

use crate::disasm::{Disassembly, format_operand};
use crate::xref::{ReferenceKind, build_cross_references};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    Subroutine,
    Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub address: u16,
    pub name: String,
    pub kind: LabelKind,
}

/// Generates labels for statically discovered control-flow destinations.
pub fn generate_labels(disassembly: &Disassembly) -> BTreeMap<u16, Label> {
    let instruction_addresses: BTreeSet<u16> = disassembly
        .instructions
        .iter()
        .map(|instruction| instruction.address)
        .collect();

    let references = build_cross_references(disassembly);
    let mut labels = BTreeMap::new();

    for reference in references {
        // Only label destinations that were actually decoded.
        if !instruction_addresses.contains(&reference.to) {
            continue;
        }

        let kind = match reference.kind {
            ReferenceKind::Call => LabelKind::Subroutine,
            ReferenceKind::Jump | ReferenceKind::Branch => LabelKind::Location,
        };

        let name = match kind {
            LabelKind::Subroutine => format!("sub_{:04X}", reference.to),
            LabelKind::Location => format!("loc_{:04X}", reference.to),
        };

        labels
            .entry(reference.to)
            .and_modify(|existing: &mut Label| {
                // A subroutine label has priority over a location label.
                if kind == LabelKind::Subroutine {
                    existing.kind = LabelKind::Subroutine;
                    existing.name = name.clone();
                }
            })
            .or_insert(Label {
                address: reference.to,
                name,
                kind,
            });
    }

    labels
}

/// Formats disassembly with generated labels and symbolic control-flow operands.
pub fn format_listing_with_labels(
    disassembly: &Disassembly,
    labels: &BTreeMap<u16, Label>,
) -> String {
    let mut output = String::new();

    for instruction in &disassembly.instructions {
        if let Some(label) = labels.get(&instruction.address) {
            output.push_str(&format!("{}:\n", label.name));
        }

        let bytes = instruction
            .bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        let mut operand = format_operand(instruction);

        if let Some(target) = instruction.target {
            if let Some(label) = labels.get(&target) {
                let address = format!("${target:04X}");
                operand = operand.replacen(&address, &label.name, 1);
            }
        }

        let marker = if instruction.unofficial { "*" } else { " " };

        output.push_str(&format!(
            "${:04X}  {:<8}{} {:<4} {}\n",
            instruction.address,
            bytes,
            marker,
            format!("{:?}", instruction.mnemonic),
            operand
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disasm::disassemble_nrom;

    #[test]
    fn creates_subroutine_and_location_labels() {
        let mut prg = vec![0xEA; 0x8000];

        // $8000: JSR $8010
        // $8003: BNE $8007
        // $8005: RTS
        // $8007: RTS
        // $8010: RTS
        prg[0x0000..0x0008].copy_from_slice(&[0x20, 0x10, 0x80, 0xD0, 0x02, 0x60, 0xEA, 0x60]);
        prg[0x0010] = 0x60;

        prg[0x7FFA..0x8000].copy_from_slice(&[0x00, 0x80, 0x00, 0x80, 0x00, 0x80]);

        let disassembly = disassemble_nrom(&prg);
        let labels = generate_labels(&disassembly);

        assert_eq!(labels.get(&0x8010).unwrap().name, "sub_8010");

        assert_eq!(labels.get(&0x8007).unwrap().name, "loc_8007");
    }

    #[test]
    fn listing_uses_symbolic_labels() {
        let mut prg = vec![0xEA; 0x8000];

        // $8000: JSR $8010
        // $8003: RTS
        // $8010: RTS
        prg[0x0000..0x0004].copy_from_slice(&[0x20, 0x10, 0x80, 0x60]);
        prg[0x0010] = 0x60;

        prg[0x7FFA..0x8000].copy_from_slice(&[0x00, 0x80, 0x00, 0x80, 0x00, 0x80]);

        let disassembly = disassemble_nrom(&prg);
        let labels = generate_labels(&disassembly);
        let listing = format_listing_with_labels(&disassembly, &labels);

        assert!(listing.contains("sub_8010:"));
        assert!(listing.contains("JSR  sub_8010"));
    }
}
