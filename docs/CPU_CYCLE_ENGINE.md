# Cycle-stepped CPU execution

The `nrs_cpu::cycle` module provides `CycleCpu`. Each call to
`CycleCpu::clock(&mut bus)` performs exactly one externally visible CPU bus
transaction and advances one CPU cycle. The complete in-flight microstate is
available through `CycleCpuState` and can be restored mid-instruction.

## Implemented sequences

- Opcode fetch as its own bus cycle.
- Immediate, zero-page, zero-page indexed, absolute, absolute indexed,
  indexed-indirect, and indirect-indexed read/write addressing.
- Indexed-read page-cross penalties and indexed-store dummy reads.
- Relative branches, including taken and page-cross cycles.
- Loads, stores, comparisons, logical operations, ADC/SBC, register transfers,
  register increments/decrements, and flag operations.
- Memory read-modify-write sequences for zero-page, zero-page indexed,
  absolute, absolute-X, indexed-indirect and indirect-indexed addressing,
  including the old-value write followed by the modified-value write.
- Accumulator ASL/LSR/ROL/ROR and unofficial combined RMW operations
  SLO/RLA/SRE/RRA/DCP/ISC for the implemented memory addressing forms.
- Stable unofficial ALR/ANC/ARR/AXS immediate operations and LAS absolute-Y.
- Stack operations PHA/PHP/PLA/PLP, JSR/RTS/RTI, BRK, JMP absolute and JMP
  indirect (including the original 6502 indirect-page-wrap behavior).
- Seven-cycle IRQ/NMI entry, NMI priority, vector selection and B-clear status push.
- Seven-cycle RESET sequence, stack-pointer reset decrements, reset-vector fetch and recovery from KIL/JAM.
- KIL/JAM latch and explicit fault reporting for unsupported sequences.
- Snapshot/restore in the middle of an instruction.

## Known gaps

This is an executable foundation, not a claim of full 6502/2A03 conformance.
The following work remains before the CPU can be considered complete:

- Exact IRQ polling latency and edge behavior around CLI/SEI/PLP/RTI.
- Exact power-on state, analog startup behavior and IRQ polling edge cases around CLI/SEI/PLP/RTI.
- Remaining unofficial instructions and unstable bus-value/address behavior
  (including XAA, immediate LAX, AHX, SHX, SHY, TAS and related opcodes).
- Exhaustive cycle-by-cycle validation for every opcode and addressing form.
- DMA stalls and their interaction with the system scheduler.

Unsupported sequences return an explicit `CpuFault`; they are not silently
executed as an instruction-at-once approximation.

## Cycle-level tests

Unit tests assert bus access order, addresses, values, write ordering, cycle
counts, flags, stack state, indirect-JMP wrapping, and mid-instruction snapshot
restoration for representative official and unofficial instructions.

The next validation layer must run established CPU test ROMs (including
nestest, Klaus Dormann and blargg suites) through a complete NES memory bus.
