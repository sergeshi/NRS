# NES Reverse Engineering Studio (NRS)

NRS is a Rust-based NES reverse-engineering and reconstruction environment.

## Design principles

- Lossless execution model: preserve CPU-visible behavior and hardware timing.
- Blueprint-first reconstruction: expose discovered executable behavior as editable visual graphs.
- Emulation-driven analysis: use deterministic execution traces as evidence.
- Bidirectional pipeline: connect ROM import, analysis, editable representation, and ROM output.
- Explicit uncertainty: distinguish verified machine behavior from inferred semantic labels.

## Current status

The repository is at the beginning of the Emulator Core milestone.

Implemented:
- Rust workspace foundation.
- Shared regional timing profile and rational CPU-to-PPU clock divider.
- Initial Ricoh 2A03 CPU state model and status-flag helpers.

Not implemented yet:
- CPU opcode execution and micro-operations.
- CPU/PPU buses, RAM, PPU, APU, mappers, DMA, controllers.
- ROM loading, snapshots, timeline, analysis, editor, or ROM building.

## Build and test

Run cargo fmt --all, cargo test --workspace, and cargo check --workspace.
