#[derive(Default, Debug)]
pub struct EnvelopeFeature {
    pub volume: u8,
    envelope_enabled: bool,
    envelope_add_mode: bool,
    envelope_period_load_value: u8,
    envelope_period: u8,
}

impl EnvelopeFeature {
    /// Tick Envelope following this specification:
    ///
    /// A volume envelope has a volume counter and an internal timer clocked at 64 Hz by the frame sequencer.
    /// When the timer generates a clock and the envelope period is not zero,
    /// a new volume is calculated by adding or subtracting (as set by NRx2) one from the current volume.
    /// If this new volume within the 0 to 15 range, the volume is updated,
    /// otherwise it is left unchanged and no further automatic
    /// increments/decrements are made to the volume until the channel is triggered again.
    ///
    /// When the waveform input is zero the envelope outputs zero, otherwise it outputs the current volume.
    pub fn tick(&mut self) {
        if self.envelope_enabled && self.envelope_period > 0 {
            self.envelope_period = self.envelope_period.saturating_sub(1);

            if self.envelope_period == 0 {
                if self.envelope_add_mode {
                    let new_val = self.volume + 1;
                    if new_val <= 15 {
                        self.volume = new_val;
                        self.envelope_period = self.envelope_period_load_value;
                    } else {
                        self.envelope_enabled = false;
                    }
                } else {
                    let (new_val, overflow) = self.volume.overflowing_sub(1);
                    if !overflow {
                        self.volume = new_val;
                        self.envelope_period = self.envelope_period_load_value;
                    } else {
                        self.envelope_enabled = false;
                    }
                }
            }
        }
    }

    /// Follows the behaviour when a channel is triggered, specifically for the Envelope feature.
    pub fn trigger(&mut self) {
        self.envelope_enabled = true;
        self.envelope_period = self.envelope_period_load_value;
    }

    pub fn read_register(&self) -> u8 {
        (self.volume << 4) | self.envelope_period_load_value | if self.envelope_add_mode { 0x8 } else { 0 }
    }

    pub fn write_register(&mut self, value: u8) {
        self.volume = (value & 0xF0) >> 4;
        self.envelope_add_mode = (value & 0x8) == 0x8;
        self.envelope_period_load_value = value & 0x7;
        // Does this immediately load?
        self.envelope_period = self.envelope_period_load_value;
    }
}

pub struct SweepFeature {
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    // Internal Sweep
    sweep_enabled: bool,
    sweep_timer: u8,
    sweep_frequency_shadow: u16,
}

impl SweepFeature {
    pub fn tick_sweep(&mut self) {
        if self.sweep_enabled && self.sweep_period != 0 {
            let temp_freq = self.sweep_calculations();
            // Duplicate overflow check, but this gets called, at most 128 times per second so, eh.
            if temp_freq < 2048 && self.sweep_shift != 0 {
                self.sweep_frequency_shadow = temp_freq;
                self._frequency = temp_freq;
                self.sweep_calculations();
            }
        }
    }
}