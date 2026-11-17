# 2A03 opcode decoder

The CPU crate exposes a fixed 256-entry opcode matrix through `OPCODES`,
`opcode_table()`, and `decode(u8)`.

## Metadata contract

Each entry records:

- opcode byte and mnemonic;
- addressing mode and encoded instruction length;
- documented base cycle count;
- whether the encoding is unofficial;
- whether a read instruction may incur an indexed page-cross cycle;
- whether it is a conditional branch;
- whether it is a KIL/JAM encoding.

The decoder is total: every possible byte from `0x00` through `0xFF`
maps to exactly one entry. The table includes the unofficial encodings used
by the NES Ricoh 2A03, including KIL, composite read/modify/write operations,
unofficial NOPs, and unstable operations such as XAA/AHX/TAS/SHX/SHY.

## Important timing distinction

`base_cycles` is not a promise that every execution takes exactly that many
cycles. Branches add a taken-branch cycle and may add another cycle when the
target crosses a page. Indexed read instructions may add a page-cross cycle.
Stores and read/modify/write instructions have their own fixed bus sequences.
The upcoming micro-operation engine, not this metadata table, is the authority
for actual bus activity and per-cycle completion.

## Validation

Unit tests assert:

1. all 256 input values decode;
2. each table slot contains the matching opcode byte;
3. all instruction lengths agree with addressing modes;
4. official/unofficial and KIL classifications are represented;
5. branch and indexed-read page-cross metadata are classified.

These are decoder completeness tests, not a substitute for established
functional and timing ROM suites. Before calling the CPU implementation
hardware-accurate, validate it against nestest, Klaus Dormann's 6502 tests,
blargg's NES CPU tests, and dedicated unofficial-opcode/timing cases.

## Next execution milestone

Implement a resumable micro-state machine. Each `clock()` call must perform
one CPU bus cycle, with explicit fetch, operand, dummy access, stack, vector,
and read/modify/write phases. The in-flight state must be snapshot-safe.
