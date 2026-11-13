# NES Reverse Engineering Studio (NRS)

NRS is a Rust-based NES reverse-engineering and reconstruction environment.

## Design principles

- Lossless execution model: preserve CPU-visible behavior and hardware timing.
- Blueprint-first reconstruction: expose discovered executable behavior as editable visual graphs.
- Emulation-driven analysis: use deterministic execution traces as evidence.
- Bidirectional pipeline: connect ROM import, analysis, editable representation, and ROM output.
- Explicit uncertainty: distinguish verified machine behavior from inferred semantic labels.

## Current status

NRS is in the Emulator Core foundation stage. It is not yet an alpha release.

Implemented:
- Rust workspace foundation and shared NTSC/PAL/Dendy timing profiles.
- Rational CPU-to-PPU clock divider with snapshotable phase.
- Ricoh 2A03 256-entry opcode metadata/decoder.
- Cycle-stepped CPU executor covering common read/write/branch sequences and memory read-modify-write microsequences, including unofficial combined RMW families for implemented addressing modes.
- iNES 1.0 and NES 2.0 ROM image parser with trainer, mapper/submapper, PRG/CHR and RAM metadata.

Not implemented:
- Complete 2A03 instruction microsequences, interrupts, DMA and hardware-accurate reset.
- CPU/PPU/APU buses, PPU rendering/register behavior, mappers and controllers.
- Machine scheduler, deterministic whole-machine snapshots and timeline.
- ROM disassembly/analysis, Blueprint IR and graph editor.
- Desktop application, debugger UI, ROM rebuilding and standalone export.

## Alpha acceptance target

The first alpha milestone is a usable vertical slice:
1. Open and validate iNES/NES 2.0 ROMs.
2. Run a deterministic NES machine with CPU, PPU, mapper, memory bus and controller input.
3. Inspect CPU registers, disassembly, memory and per-cycle bus traces.
4. Pause, single-step, set breakpoints and capture/restore machine snapshots.
5. Save a project with ROM identity, analysis metadata and snapshots.
6. Run a repeatable automated test suite, including established CPU and PPU test ROMs.

A feature is not considered complete merely because its data structures or opcode metadata exist. It must execute and be covered by tests.

## Build and test

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
```
