# Emulator Core — architecture baseline

## Scope

This milestone establishes the Rust workspace, regional rational CPU-to-PPU
clock progression, and an initial Ricoh 2A03 CPU state container. It is not yet
a working NES emulator.

## Planned crate boundaries

- nrs-core: shared timing, configuration, errors, and snapshot primitives.
- nrs-cpu: 2A03 state, opcode metadata, cycle-level micro-operations, interrupts.
- nrs-memory: CPU/PPU buses, RAM, open-bus behavior, and device routing.
- nrs-ppu: registers, rendering pipeline, VRAM/OAM, and NMI timing.
- nrs-apu: pulse, triangle, noise, DMC, frame sequencer, and audio output.
- nrs-mappers: cartridge boards and mapper behavior.
- nrs-emulator: deterministic scheduler, input events, snapshots, and run loop.

## Timing model

The scheduler must not use floating-point time to coordinate hardware.
CPU-to-PPU progression uses a rational divider whose phase is snapshot state.
Initial ratios are 3:1 for NTSC and Dendy and 16:5 for PAL.

These ratios are only the initial regional scheduler contract. Odd-frame
behavior, CPU/APU phase relationships, reset timing, and board-specific quirks
must be validated against test ROMs and hardware references.

## CPU execution contract

The CPU will advance one hardware cycle at a time. Every externally visible
read or write must occur on its corresponding cycle. An instruction-level
interpreter that performs all memory accesses atomically is insufficient.

CPU micro-operation state must be serializable so snapshots can resume in the
middle of an instruction. Bus side effects, page-cross penalties, dummy reads,
interrupt polling, and DMA stalls must be represented explicitly.

## Reverse-engineering contract

The emulator is the behavioral oracle for dynamic analysis. Traces must
preserve address, value, access kind, cycle, and device side effects. Analysis
may attach semantic hypotheses to trace regions, but must never confuse
inferred names with original source symbols.
