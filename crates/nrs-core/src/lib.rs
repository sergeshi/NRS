#![forbid(unsafe_code)]

//! Shared hardware timing and foundational types for NRS.

/// NES hardware timing region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    /// NTSC timing.
    Ntsc,
    /// PAL timing.
    Pal,
    /// Dendy timing.
    Dendy,
}

/// Regional timing parameters used by the emulator scheduler.
///
/// The CPU-to-PPU ratio is represented as a rational number so PAL timing
/// does not accumulate floating-point drift. This is an initial scheduler
/// contract; regional frequencies and hardware-specific details require
/// validation against test ROMs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingProfile {
    /// Region represented by this profile.
    pub region: Region,
    /// PPU ticks per CPU tick numerator.
    pub ppu_per_cpu_num: u16,
    /// PPU ticks per CPU tick denominator.
    pub ppu_per_cpu_den: u16,
    /// Number of scanlines in a frame.
    pub scanlines: u16,
    /// PPU dots per scanline.
    pub dots_per_scanline: u16,
    /// Whether the PPU skips a dot on an odd rendered NTSC frame.
    pub odd_frame_skip: bool,
}

impl TimingProfile {
    /// Initial NTSC timing profile.
    pub const NTSC: Self = Self {
        region: Region::Ntsc,
        ppu_per_cpu_num: 3,
        ppu_per_cpu_den: 1,
        scanlines: 262,
        dots_per_scanline: 341,
        odd_frame_skip: true,
    };

    /// Initial PAL timing profile.
    pub const PAL: Self = Self {
        region: Region::Pal,
        ppu_per_cpu_num: 16,
        ppu_per_cpu_den: 5,
        scanlines: 312,
        dots_per_scanline: 341,
        odd_frame_skip: false,
    };

    /// Initial Dendy timing profile.
    pub const DENDY: Self = Self {
        region: Region::Dendy,
        ppu_per_cpu_num: 3,
        ppu_per_cpu_den: 1,
        scanlines: 312,
        dots_per_scanline: 341,
        odd_frame_skip: false,
    };

    /// Returns the initial timing profile for a region.
    pub const fn for_region(region: Region) -> Self {
        match region {
            Region::Ntsc => Self::NTSC,
            Region::Pal => Self::PAL,
            Region::Dendy => Self::DENDY,
        }
    }

    /// Validates the ratio before constructing a clock divider.
    pub const fn has_valid_clock_ratio(self) -> bool {
        self.ppu_per_cpu_num > 0 && self.ppu_per_cpu_den > 0
    }
}

/// Rational divider converting CPU ticks into PPU ticks.
///
/// The accumulator is part of emulator state and must be included in
/// save states to resume at the same relative clock phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PpuClockDivider {
    numerator: u16,
    denominator: u16,
    accumulator: u32,
}

impl PpuClockDivider {
    /// Creates a divider from a timing profile.
    ///
    /// # Panics
    ///
    /// Panics if the profile has a zero numerator or denominator.
    pub fn new(profile: TimingProfile) -> Self {
        assert!(profile.has_valid_clock_ratio(), "invalid CPU/PPU clock ratio");
        Self {
            numerator: profile.ppu_per_cpu_num,
            denominator: profile.ppu_per_cpu_den,
            accumulator: 0,
        }
    }

    /// Advances one CPU tick and returns the number of PPU ticks due.
    pub fn advance_cpu_tick(&mut self) -> u16 {
        self.accumulator += u32::from(self.numerator);
        let ticks = self.accumulator / u32::from(self.denominator);
        self.accumulator %= u32::from(self.denominator);
        // The accumulator starts below the denominator and adds at most one
        // u16 numerator, so the quotient cannot exceed u16::MAX.
        ticks as u16
    }

    /// Returns the fractional phase currently held by the divider.
    pub const fn phase(&self) -> u32 {
        self.accumulator
    }

    /// Restores a previously saved fractional phase.
    ///
    /// Returns false when the phase is outside this divider's denominator.
    pub fn restore_phase(&mut self, phase: u32) -> bool {
        if phase >= u32::from(self.denominator) {
            return false;
        }
        self.accumulator = phase;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntsc_emits_three_ppu_ticks_per_cpu_tick() {
        let mut divider = PpuClockDivider::new(TimingProfile::NTSC);
        for _ in 0..100 {
            assert_eq!(divider.advance_cpu_tick(), 3);
        }
    }

    #[test]
    fn pal_ratio_is_exact_over_five_cpu_ticks() {
        let mut divider = PpuClockDivider::new(TimingProfile::PAL);
        let total: u16 = (0..5).map(|_| divider.advance_cpu_tick()).sum();
        assert_eq!(total, 16);
        assert_eq!(divider.phase(), 0);
    }

    #[test]
    fn divider_phase_can_be_restored() {
        let mut original = PpuClockDivider::new(TimingProfile::PAL);
        for _ in 0..3 {
            original.advance_cpu_tick();
        }
        let phase = original.phase();
        let mut restored = PpuClockDivider::new(TimingProfile::PAL);
        assert!(restored.restore_phase(phase));
        assert_eq!(restored.phase(), phase);
        assert!(!restored.restore_phase(5));
    }
}
