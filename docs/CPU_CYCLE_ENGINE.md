# Cycle-stepped CPU execution

The `nrs_cpu::cycle` module introduces `CycleCpu`. Each call to
`CycleCpu::clock(&mut bus)` performs one bus transaction and advances one
CPU cycle. The complete in-flight microstate is available through
`CycleCpuState` and can be restored to resume an instruction.

## Implemented in this increment

- Opcode fetch as its own bus cycle.
- Immediate, zero-page, zero-page indexed, absolute, absolute indexed,
  indexed-indirect, and indirect-indexed addressing sequences.
- Correct indexed-read page-cross extra cycles and indexed-store dummy reads.
- Relative branches, including taken and page-cross cycles.
- Common register transfers, increments/decrements, flag operations,
  loads, stores, comparisons, logical operations, ADC and SBC.
- Memory RMW sequences for zero-page, zero-page indexed, absolute,
  absolute-X, indexed-indirect and indirect-indexed forms, including the
  old-value write followed by the modified-value write.
- Accumulator ASL/LSR/ROL/ROR and unofficial combined RMW operations
  SLO/RLA/SRE/RRA/DCP/ISC for the implemented memory addressing forms.
- KIL/JAM latch and explicit CPU fault reporting.
- Snapshot/restore in the middle of an instruction.

## Explicitly not complete yet

This is an executable foundation, not yet a claim of full 6502/2A03 conformance.
The following sequences still need implementation before the CPU can be called
complete:

- Stack and control-flow instructions: BRK, JSR, RTS, RTI, JMP indirect,
  PHA/PHP/PLA/PLP.
- Remaining opcode-specific edge cases and exhaustive bus traces for every RMW addressing form.
- Remaining unofficial operations, including unstable bus-value behavior.
- Exact IRQ/NMI/RESET entry and interrupt polling timing.
- DMA stalls and interaction with the system scheduler.
- Hardware-accurate power-on/reset behavior.

Unsupported sequences return an explicit `CpuFault`; they are not silently
executed as an instruction-at-once approximation.

## Cycle-level test contract

Tests assert the number and address/value of bus transactions for immediate,
zero-page stores, indexed page-cross reads, zero-page and absolute-X RMW,
accumulator shifts, unofficial indexed-indirect/indirect-indexed RMW, and
mid-instruction snapshot cases. The next validation layer must add per-opcode bus traces and run
nestest, Klaus Dormann, blargg, and unofficial-opcode timing suites.
