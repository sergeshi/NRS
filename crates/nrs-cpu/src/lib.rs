#![forbid(unsafe_code)]

//! Initial state model for the NES Ricoh 2A03 CPU.
//!
//! This crate does not execute instructions yet. The next CPU milestone
//! will add the opcode table and cycle-by-cycle micro-operation engine.

/// Bit masks for the 6502 status register.
pub mod flags {
    /// Carry flag.
    pub const CARRY: u8 = 0b0000_0001;
    /// Zero flag.
    pub const ZERO: u8 = 0b0000_0010;
    /// Interrupt-disable flag.
    pub const INTERRUPT_DISABLE: u8 = 0b0000_0100;
    /// Decimal flag (the 2A03 ADC/SBC datapath ignores decimal mode).
    pub const DECIMAL: u8 = 0b0000_1000;
    /// Break bit as represented by PHP/BRK stack images.
    pub const BREAK: u8 = 0b0001_0000;
    /// Unused status bit; bit 5 reads as one internally.
    pub const UNUSED: u8 = 0b0010_0000;
    /// Overflow flag.
    pub const OVERFLOW: u8 = 0b0100_0000;
    /// Negative flag.
    pub const NEGATIVE: u8 = 0b1000_0000;
}

/// Architectural and in-flight CPU state.
///
/// Fields are public in this first milestone for inspection and tests.
/// Snapshot format stability will be formalized separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuState {
    /// Accumulator.
    pub a: u8,
    /// X index register.
    pub x: u8,
    /// Y index register.
    pub y: u8,
    /// Stack pointer.
    pub stack_pointer: u8,
    /// Program counter.
    pub program_counter: u16,
    /// Processor status register.
    pub status: u8,
    /// Current opcode latch.
    pub opcode: u8,
    /// Current micro-operation index.
    pub micro_cycle: u8,
    /// Total CPU ticks elapsed.
    pub total_cycles: u64,
    /// Latched NMI request.
    pub nmi_pending: bool,
    /// Current IRQ input level.
    pub irq_line: bool,
}

impl Default for CpuState {
    fn default() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            stack_pointer: 0xFD,
            program_counter: 0,
            status: flags::INTERRUPT_DISABLE | flags::UNUSED,
            opcode: 0,
            micro_cycle: 0,
            total_cycles: 0,
            nmi_pending: false,
            irq_line: false,
        }
    }
}

/// External bus interface required by the CPU.
///
/// Reads and writes are issued by the CPU micro-operation engine. The bus
/// implementation is responsible for memory mapping and device side effects.
pub trait CpuBus {
    /// Performs one CPU-visible bus read.
    fn read(&mut self, address: u16) -> u8;

    /// Performs one CPU-visible bus write.
    fn write(&mut self, address: u16, value: u8);
}

/// Ricoh 2A03 CPU state container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cpu {
    state: CpuState,
}

impl Cpu {
    /// Creates a CPU state container in its initial placeholder state.
    ///
    /// Hardware-accurate power-on behavior and reset-vector fetching are
    /// deferred to the reset-sequencing milestone.
    pub const fn new() -> Self {
        Self {
            state: CpuState {
                a: 0,
                x: 0,
                y: 0,
                stack_pointer: 0xFD,
                program_counter: 0,
                status: flags::INTERRUPT_DISABLE | flags::UNUSED,
                opcode: 0,
                micro_cycle: 0,
                total_cycles: 0,
                nmi_pending: false,
                irq_line: false,
            },
        }
    }

    /// Returns a copy of the complete current CPU state.
    pub const fn state(&self) -> CpuState {
        self.state
    }

    /// Restores a previously captured CPU state.
    pub fn restore_state(&mut self, mut state: CpuState) {
        state.status |= flags::UNUSED;
        self.state = state;
    }

    /// Sets the accumulator. Intended for debugger setup and tests.
    pub fn set_register_a(&mut self, value: u8) {
        self.state.a = value;
    }

    /// Sets the X register. Intended for debugger setup and tests.
    pub fn set_register_x(&mut self, value: u8) {
        self.state.x = value;
    }

    /// Sets the Y register. Intended for debugger setup and tests.
    pub fn set_register_y(&mut self, value: u8) {
        self.state.y = value;
    }

    /// Sets the program counter. Intended for debugger setup and tests.
    pub fn set_program_counter(&mut self, value: u16) {
        self.state.program_counter = value;
    }

    /// Returns whether a status flag is set.
    pub const fn flag(&self, mask: u8) -> bool {
        self.state.status & mask != 0
    }

    /// Sets or clears a status flag.
    pub fn set_flag(&mut self, mask: u8, enabled: bool) {
        if enabled {
            self.state.status |= mask;
        } else {
            self.state.status &= !mask;
        }
        self.state.status |= flags::UNUSED;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_starts_with_initial_placeholder_state() {
        let cpu = Cpu::new();
        let state = cpu.state();
        assert_eq!(state.a, 0);
        assert_eq!(state.x, 0);
        assert_eq!(state.y, 0);
        assert_eq!(state.stack_pointer, 0xFD);
        assert_eq!(state.program_counter, 0);
        assert!(state.status & flags::INTERRUPT_DISABLE != 0);
        assert!(state.status & flags::UNUSED != 0);
    }

    #[test]
    fn status_flags_can_be_changed() {
        let mut cpu = Cpu::new();
        cpu.set_flag(flags::CARRY, true);
        assert!(cpu.flag(flags::CARRY));
        cpu.set_flag(flags::CARRY, false);
        assert!(!cpu.flag(flags::CARRY));
        assert!(cpu.flag(flags::UNUSED));
    }

    #[test]
    fn cpu_state_can_be_restored_for_snapshot_work() {
        let mut cpu = Cpu::new();
        let mut state = cpu.state();
        state.a = 42;
        state.program_counter = 0x8123;
        state.status &= !flags::UNUSED;
        cpu.restore_state(state);
        assert_eq!(cpu.state().a, 42);
        assert_eq!(cpu.state().program_counter, 0x8123);
        assert!(cpu.flag(flags::UNUSED));
    }
}
