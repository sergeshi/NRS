# Ricoh 2A03 CPU execution contract

Status: design contract for the cycle-stepped CPU implementation. This document is not a claim that the CPU core is already implemented.

## Scope

The target is the NES Ricoh 2A03/2A07: NMOS 6502 execution semantics with the NES-specific disabled decimal arithmetic datapath. The implementation must cover all 256 opcode bytes, including stable undocumented instructions, multi-byte NOPs, KIL/JAM, and the unstable store/load family with an explicitly documented deterministic policy.

## Non-negotiable execution model

- `Cpu::clock(&mut self, bus: &mut impl CpuBus)` advances exactly one CPU cycle.
- A clock performs at most one externally visible CPU bus access. Dummy reads and dummy writes are real accesses, not elapsed-cycle counters.
- `Cpu::step_instruction` is only a debugger convenience wrapper over repeated `clock` calls; it must not have a separate execution implementation.
- The in-flight instruction state is snapshot data. Restoring a snapshot between any two bus cycles must resume identically.
- The bus owns memory mapping and side effects. CPU code must not peek into RAM or bypass the bus.
- Opcode metadata is declarative and separate from the microsequencer. Disassembly/Blueprint tooling consumes the same metadata but cannot change execution semantics.

## Per-cycle state

The serializable state must include at least:

- A, X, Y, S, PC, P; opcode and fetched operand latches.
- Effective address, base address, provisional address, page-cross flag, data latch and branch target.
- Current instruction/micro-step, interrupt sequence kind and vector selection.
- NMI edge latch, IRQ level, sampled interrupt-disable state and interrupt-poll phase.
- Reset/halt state and total elapsed CPU cycles.

Status bit 5 is forced high in register images. B is not a persistent physical status latch: PHP/BRK push B=1; IRQ/NMI push B=0. The 2A03 ignores D for ADC/SBC while preserving the D flag bit.

## Bus-cycle sequencing

Each addressing mode is represented as an ordered sequence of micro-operations. The following are minimum contracts:

| Family | Required bus sequence |
|---|---|
| Implied / accumulator | opcode fetch; dummy read at next PC; execute register operation |
| Immediate | opcode fetch; operand read; execute |
| Zero page | opcode fetch; address byte; data read/write |
| Zero page indexed | opcode fetch; base byte; dummy read at unindexed zero-page address; indexed access |
| Absolute | opcode fetch; low byte; high byte; data access |
| Absolute indexed read | opcode fetch; low; high; provisional read; if page crossed, corrected read |
| Absolute indexed write | opcode fetch; low; high; unconditional provisional dummy read; corrected write |
| Indexed-indirect (X) | opcode fetch; zero-page operand; dummy read; pointer low; pointer high with zero-page wrap; final access |
| Indirect-indexed (Y) read | opcode fetch; zero-page operand; pointer low; pointer high with zero-page wrap; provisional read; corrected read only on page crossing |
| Indirect-indexed (Y) write | same pointer fetch; unconditional provisional dummy read; corrected write |
| Read-modify-write | read old value; write old value; write modified value; dummy cycles retained |
| Branch | displacement read; taken-branch dummy read; extra wrong-page read on page crossing |
| JMP indirect | pointer low/high reads; target low; target high using the NMOS page-wrap bug |
| JSR / RTS / RTI | stack dummy accesses and each stack read/write represented as individual cycles |
| BRK / IRQ / NMI | padding read, three stack writes, vector low/high reads; B bit differs by source |
| Reset | seven-cycle reset sequence with stack-pointer decrements and vector reads |

The exact bus address for each dummy cycle is part of the behavior and must be asserted by bus-trace tests.

## Opcode table

The decoder must be total: every byte 0x00..0xFF maps to a descriptor. A descriptor contains mnemonic/operation, addressing mode, instruction class, length, and a stable semantic identifier. Nominal cycles are descriptive only; dynamic execution is determined by micro-operations and bus conditions.

Unofficial opcodes are grouped as:

- Composite RMW: SLO, RLA, SRE, RRA, DCP, ISC/ISB.
- Register transfer/load-store: SAX, LAX, LAS.
- Immediate ALU: ANC, ALR, ARR, AXS/SBX, XAA/ANE and duplicate SBC.
- Unstable stores: AHX/SHA, SHX, SHY, TAS.
- NOP variants with their real operand reads and cycle counts.
- KIL/JAM opcodes that halt until reset.

Unstable opcodes must have a documented policy and dedicated tests; they must not silently alias to NOP.

## Interrupt timing

NMI is edge-latched. IRQ is level-sensitive. Interrupt recognition is sampled at the hardware-defined polling point, not simply checked before every instruction. CLI, SEI, PLP, RTI, branch timing, and NMI hijacking of BRK/IRQ require dedicated regression tests. IRQ/NMI entry must be represented by the same per-cycle bus engine as normal instructions.

## Validation gates

1. Unit tests for every addressing mode, ALU flag edge case, stack operation, wraparound, and page-cross condition.
2. Bus-trace tests asserting address, direction, value, and cycle index for representative instructions in every family.
3. SingleStepTests 6502/NES JSON vectors for instruction state and memory effects.
4. nestest register/cycle log.
5. Klaus Dormann functional tests and NES-specific CPU test ROMs.
6. AccuracyCoin CPU cases, including unofficial opcodes and interrupt edge cases.
7. Snapshot/restore equivalence at every micro-step: uninterrupted execution and save/restore execution must produce identical state and bus trace.

## Implementation order

1. Opcode descriptor table and exhaustive decode tests.
2. Bus-cycle state machine and reset/fetch sequencing.
3. Addressing-mode microsequences.
4. Official read/write/branch/stack/control-flow operations.
5. RMW and all stable undocumented operations.
6. IRQ/NMI polling and vector hijacking.
7. Unstable opcodes, KIL behavior, snapshot equivalence, ROM-based validation.

A milestone is not complete merely because all opcode bytes decode or instruction-level register tests pass. Cycle-level bus traces and external ROM suites are required.
