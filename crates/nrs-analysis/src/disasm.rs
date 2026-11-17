use nrs_cpu::{AddressingMode, Mnemonic, decode};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub mnemonic: Mnemonic,
    pub mode: AddressingMode,
    pub target: Option<u16>,
    pub unofficial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Disassembly {
    pub instructions: Vec<Instruction>,
    pub unresolved_indirect_jumps: Vec<u16>,
    pub invalid_targets: Vec<u16>,
}

impl Disassembly {
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    pub fn format_listing(&self) -> String {
        let mut output = String::new();

        for instruction in &self.instructions {
            let bytes = instruction
                .bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ");

            let operand = format_operand(instruction);
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
}

/// Converts a CPU address into an offset inside NROM PRG ROM.
fn prg_offset(address: u16, prg_len: usize) -> Option<usize> {
    if address < 0x8000 || prg_len == 0 {
        return None;
    }

    let mut offset = (address - 0x8000) as usize;

    // NROM-128 mirrors its 16 KiB PRG bank.
    if prg_len == 0x4000 {
        offset &= 0x3FFF;
    }

    if offset < prg_len { Some(offset) } else { None }
}

fn read_byte(prg: &[u8], address: u16) -> Option<u8> {
    prg_offset(address, prg.len()).map(|offset| prg[offset])
}

fn relative_target(address: u16, displacement: u8) -> u16 {
    let next = address.wrapping_add(2);
    next.wrapping_add((displacement as i8 as i16) as u16)
}

pub(crate) fn format_operand(instruction: &Instruction) -> String {
    let byte = |index: usize| instruction.bytes.get(index).copied().unwrap_or(0);

    let word = u16::from_le_bytes([byte(1), byte(2)]);

    match instruction.mode {
        AddressingMode::Implied => String::new(),
        AddressingMode::Accumulator => "A".into(),
        AddressingMode::Immediate => format!("#${:02X}", byte(1)),
        AddressingMode::ZeroPage => format!("${:02X}", byte(1)),
        AddressingMode::ZeroPageX => format!("${:02X},X", byte(1)),
        AddressingMode::ZeroPageY => format!("${:02X},Y", byte(1)),
        AddressingMode::Absolute => format!("${word:04X}"),
        AddressingMode::AbsoluteX => format!("${word:04X},X"),
        AddressingMode::AbsoluteY => format!("${word:04X},Y"),
        AddressingMode::Indirect => format!("(${word:04X})"),
        AddressingMode::IndexedIndirect => format!("(${:02X},X)", byte(1)),
        AddressingMode::IndirectIndexed => format!("(${:02X}),Y", byte(1)),
        AddressingMode::Relative => {
            let target = instruction.target.unwrap_or(0);
            format!("${target:04X}")
        }
    }
}

/// Performs conservative recursive traversal of reachable NROM code.
///
/// Entry points:
/// - RESET vector
/// - NMI vector
/// - IRQ/BRK vector
///
/// Conditional branches are followed in both directions.
/// Absolute JMP and JSR destinations are added to the traversal queue.
/// Indirect JMP destinations cannot be resolved without runtime memory state.
///
/// Supported PRG sizes:
/// - 16 KiB (NROM-128)
/// - 32 KiB (NROM-256)
pub fn disassemble_nrom(prg: &[u8]) -> Disassembly {
    let mut result = Disassembly::default();

    if prg.len() != 0x4000 && prg.len() != 0x8000 {
        return result;
    }

    let Some(vectors) = read_vectors(prg) else {
        return result;
    };

    let mut queue = VecDeque::new();
    let mut visited = BTreeSet::new();
    let mut instructions = BTreeMap::new();

    // Add interrupt handlers as independent entry points.
    for address in [vectors[1], vectors[0], vectors[2]] {
        if prg_offset(address, prg.len()).is_some() {
            queue.push_back(address);
        } else {
            result.invalid_targets.push(address);
        }
    }

    // Records which instruction owns each physical PRG byte.
    let mut owners = vec![None::<u16>; prg.len()];

    while let Some(mut pc) = queue.pop_front() {
        loop {
            let Some(offset) = prg_offset(pc, prg.len()) else {
                result.invalid_targets.push(pc);
                break;
            };

            // Do not decode an address more than once.
            if !visited.insert(pc) {
                break;
            }

            // This physical byte already belongs to another instruction.
            if owners[offset].is_some() {
                break;
            }

            let Some(opcode_byte) = read_byte(prg, pc) else {
                break;
            };

            let opcode = decode(opcode_byte);
            let length = opcode.bytes as usize;

            // Read the complete instruction.
            let mut bytes = Vec::with_capacity(length);
            let mut complete = true;

            for index in 0..length {
                let address = pc.wrapping_add(index as u16);

                if let Some(value) = read_byte(prg, address) {
                    bytes.push(value);
                } else {
                    complete = false;
                    break;
                }
            }

            if !complete {
                break;
            }

            // Avoid interpreting bytes already owned by another instruction.
            let overlaps = (0..length).any(|index| {
                let address = pc.wrapping_add(index as u16);

                prg_offset(address, prg.len())
                    .and_then(|i| owners.get(i))
                    .is_none_or(|owner| owner.is_some())
            });

            if overlaps {
                break;
            }

            // Mark all bytes occupied by this instruction.
            for index in 0..length {
                let address = pc.wrapping_add(index as u16);

                if let Some(i) = prg_offset(address, prg.len()) {
                    owners[i] = Some(pc);
                }
            }

            let next = pc.wrapping_add(length as u16);

            // Resolve statically known destinations.
            let target = match opcode.mode {
                AddressingMode::Relative if opcode.is_branch => Some(relative_target(pc, bytes[1])),

                AddressingMode::Absolute
                    if matches!(opcode.mnemonic, Mnemonic::JMP | Mnemonic::JSR) =>
                {
                    Some(u16::from_le_bytes([bytes[1], bytes[2]]))
                }

                _ => None,
            };

            // Store the decoded instruction.
            instructions.insert(
                pc,
                Instruction {
                    address: pc,
                    bytes,
                    mnemonic: opcode.mnemonic,
                    mode: opcode.mode,
                    target,
                    unofficial: opcode.unofficial,
                },
            );

            // Conditional branch: follow both taken and not-taken paths.
            if opcode.is_branch {
                if let Some(destination) = target {
                    queue.push_back(destination);
                }

                queue.push_back(next);
                break;
            }

            match opcode.mnemonic {
                Mnemonic::JMP => {
                    if opcode.mode == AddressingMode::Absolute {
                        if let Some(destination) = target {
                            queue.push_back(destination);
                        }
                    } else {
                        result.unresolved_indirect_jumps.push(pc);
                    }

                    break;
                }

                Mnemonic::JSR => {
                    if let Some(destination) = target {
                        queue.push_back(destination);
                    }

                    // Continue after the subroutine call as a separate path.
                    queue.push_back(next);
                    break;
                }

                Mnemonic::RTS | Mnemonic::RTI | Mnemonic::BRK | Mnemonic::KIL => {
                    break;
                }

                // Ordinary instruction: continue sequentially.
                _ => {
                    pc = next;
                }
            }
        }
    }

    // Convert the ordered map into an address-sorted instruction list.
    result.instructions = instructions.into_values().collect();

    result.unresolved_indirect_jumps.sort_unstable();
    result.unresolved_indirect_jumps.dedup();

    result.invalid_targets.sort_unstable();
    result.invalid_targets.dedup();

    result
}

/// Reads NMI, RESET and IRQ/BRK vectors from the final six PRG bytes.
fn read_vectors(prg: &[u8]) -> Option<[u16; 3]> {
    if prg.len() < 6 {
        return None;
    }

    let start = prg.len() - 6;

    let read = |offset: usize| u16::from_le_bytes([prg[start + offset], prg[start + offset + 1]]);

    Some([
        read(0), // NMI
        read(2), // RESET
        read(4), // IRQ/BRK
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_reset_vector_and_decodes_instructions() {
        let mut prg = vec![0xEA; 0x8000];

        // $8000: LDA #$42
        // $8002: JMP $8000
        prg[0] = 0xA9;
        prg[1] = 0x42;
        prg[2] = 0x4C;
        prg[3] = 0x00;
        prg[4] = 0x80;

        // NMI, RESET, IRQ vectors.
        prg[0x7FFA..0x8000].copy_from_slice(&[0x00, 0x80, 0x00, 0x80, 0x00, 0x80]);

        let result = disassemble_nrom(&prg);

        assert_eq!(result.instruction_count(), 2);
        assert_eq!(result.instructions[0].address, 0x8000);
        assert_eq!(result.instructions[1].address, 0x8002);
        assert_eq!(result.instructions[1].target, Some(0x8000));
    }

    #[test]
    fn follows_conditional_branch_in_both_directions() {
        let mut prg = vec![0xEA; 0x8000];

        // $8000: BNE $8004
        // $8002: RTS
        // $8004: RTS
        prg[0] = 0xD0;
        prg[1] = 0x02;
        prg[2] = 0x60;
        prg[4] = 0x60;

        prg[0x7FFA..0x8000].copy_from_slice(&[0x00, 0x80, 0x00, 0x80, 0x00, 0x80]);

        let result = disassemble_nrom(&prg);

        assert_eq!(result.instruction_count(), 3);
        assert_eq!(result.instructions[0].target, Some(0x8004));
    }

    #[test]
    fn recognizes_indirect_jump_as_unresolved() {
        let mut prg = vec![0xEA; 0x8000];

        // $8000: JMP ($1234)
        prg[0] = 0x6C;
        prg[1] = 0x34;
        prg[2] = 0x12;

        prg[0x7FFA..0x8000].copy_from_slice(&[0x00, 0x80, 0x00, 0x80, 0x00, 0x80]);

        let result = disassemble_nrom(&prg);

        assert_eq!(result.unresolved_indirect_jumps, vec![0x8000]);
    }

    #[test]
    fn supports_nrom_128_mirroring() {
        let mut prg = vec![0xEA; 0x4000];

        prg[0] = 0x60;

        prg[0x3FFA..0x4000].copy_from_slice(&[0x00, 0x80, 0x00, 0x80, 0x00, 0x80]);

        let result = disassemble_nrom(&prg);

        assert_eq!(result.instruction_count(), 1);
        assert_eq!(result.instructions[0].address, 0x8000);
    }
}
