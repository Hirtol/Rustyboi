use crate::hardware::apu::test_bit;

#[derive(Default, Debug, Copy, Clone)]
pub struct EnvelopeFeature {
    pub volume: u8,
    pub volume_load: u8,
    pub envelope_add_mode: bool,
    pub envelope_enabled: bool,
    pub envelope_period: u8,
    envelope_timer: u8,
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
        if self.envelope_enabled && self.envelope_period != 0 {
            self.envelope_timer = self.envelope_timer.saturating_sub(1);

            if self.envelope_timer == 0 {
                self.envelope_timer = self.envelope_period;
                if self.envelope_add_mode && self.volume < 0xF {
                    self.volume += 1;
                } else if !self.envelope_add_mode && self.volume > 0 {
                    self.volume -= 1;
                } else {
                    self.envelope_enabled = false;
                }
            }
        }
    }

    /// Follows the behaviour when a channel is triggered, specifically for the Envelope feature.
    pub fn trigger(&mut self, next_step_envelope: bool) {
        self.envelope_enabled = true;
        self.volume = self.volume_load;
        self.envelope_timer = if next_step_envelope {
            (self.envelope_period + 1) % 8
        } else {
            self.envelope_period
        };
    }

    pub fn read_register(&self) -> u8 {
        (self.volume_load << 4) | self.envelope_period | if self.envelope_add_mode { 0x8 } else { 0 }
    }

    pub fn write_register(&mut self, value: u8) {
        self.volume_load = (value & 0xF0) >> 4;
        self.envelope_add_mode = test_bit(value, 3);
        self.envelope_period = value & 0x7;
    }

    /// Checks for an obscure behaviour where the volume of the envelope feature can actually
    /// be changed while it's active under certain circumstances.
    ///
    /// A notable game which makes use of this is Prehistorik Man.
    /// The channel is expected to be triggered as a precondition for this call.
    pub fn zombie_mode_write(&mut self, old_envelope: EnvelopeFeature) {
        // Zombie mode, credits to SameBoy for figuring out this behaviour.
        if self.envelope_add_mode {
            self.volume += 1;
        }

        if old_envelope.envelope_add_mode != self.envelope_add_mode {
            self.volume = 16 - self.volume;
        }

        if self.envelope_period != 0 && old_envelope.envelope_period == 0
            && self.volume != 0 && !self.envelope_add_mode {
            self.volume -= 1;
        }

        if old_envelope.envelope_period != 0 && self.envelope_add_mode {
            self.volume -= 1;
        }

        self.volume &= 0b1111;
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct LengthFeature {
    pub length_enable: bool,
    pub length_timer: u16,
}

impl LengthFeature {
    /// Ticks the length feature.
    ///
    /// # Manipulations
    /// * `channel_enable` - Is reset when the length timer reaches 0, otherwise untouched.
    pub fn tick(&mut self, channel_enable: &mut bool) {
        if self.length_enable && self.length_timer > 0 {
            self.length_timer -= 1;

            if self.length_timer == 0 {
                *channel_enable = false;
            }
        }
    }

    /// Follows the behaviour when a channel is triggered for the Length feature (64)
    pub fn trigger(&mut self, next_step_no_length: bool) {
        if self.length_timer == 0 {
            // If a channel is triggered when the frame sequencer's next step is one that doesn't
            // clock the length counter and the length counter is now enabled and length is
            // being set to 64 because it was previously zero, it is set to 63 instead.
            if next_step_no_length && self.length_enable {
                self.length_timer = 63;
            } else {
                self.length_timer = 64;
            }
        }
    }

    pub fn write_register(&mut self, value: u8) {
        let length_load = value & 0x3F;
        self.length_timer = 64 - length_load as u16;
    }

    /// Follows the behaviour for a wave channel, Length feature (256)
    pub fn trigger_256(&mut self, next_step_no_length: bool) {
        if self.length_timer == 0 {
            // See trigger() method above, except with 256 and 255.
            if next_step_no_length && self.length_enable {
                self.length_timer = 255;
            } else {
                self.length_timer = 256;
            }
        }
    }

    pub fn write_register_256(&mut self, value: u8) {
        self.length_timer = 256 - value as u16;
    }

    /// Tests for a particular edge case (described within the method) when the address points
    /// to one of the NRx4 registers of the voices.
    pub fn second_half_enable_tick(&mut self, channel_enable: &mut bool, old_enable: bool) {
        // If we write to length when the next step in the frame sequencer DOESN't tick length
        // AND if the length counter was previously disabled and now enabled AND the length
        // counter isn't zero it is then decremented once.
        if !old_enable {
            self.tick(channel_enable);
        }
    }
}

#[derive(Default, Debug)]
pub struct SweepFeature {
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    // Internal Sweep
    sweep_enabled: bool,
    done_negate_calc: bool,
    sweep_timer: u8,
    sweep_frequency_shadow: u16,
}

impl SweepFeature {
    /// Ticks the sweep feature.
    /// Expects the channel enable and frequency
    ///
    /// # Manipulations
    /// * `channel_enable` - Is reset when the sum of the
    /// shifted shadow frequency + shadow frequency is >= 2048, otherwise untouched.
    /// * `channel_frequency` - Is set to a new value when the tick function has a shadow frequency
    /// lower than 2048.
    pub fn tick(&mut self, channel_enable: &mut bool, channel_frequency: &mut u16) {
        self.sweep_timer = self.sweep_timer.saturating_sub(1);
        if self.sweep_timer == 0 {
            if self.sweep_enabled && self.sweep_period != 0 {
                let temp_freq = self.sweep_calculations(channel_enable);
                // Duplicate overflow check, but this gets called, at most 128 times per second so, eh.
                if temp_freq < 2048 && self.sweep_shift != 0 {
                    self.sweep_frequency_shadow = temp_freq;

                    *channel_frequency = temp_freq;

                    self.sweep_calculations(channel_enable);
                }
            }
            self.sweep_timer = if self.sweep_period == 0 { 8 } else { self.sweep_period };
        }
    }

    /// Follows the behaviour when a channel is triggered, specifically for the Sweep feature.
    pub fn trigger_sweep(&mut self, channel_enable: &mut bool, frequency: u16) {
        self.sweep_frequency_shadow = frequency;
        self.sweep_enabled = self.sweep_period != 0 || self.sweep_shift != 0;
        // Sweep timer treats a period of 0 as 8 for some reason.
        self.sweep_timer = if self.sweep_period == 0 { 8 } else { self.sweep_period };
        self.done_negate_calc = false;
        //If the sweep shift is non-zero, frequency calculation and the overflow check are performed immediately.
        if self.sweep_shift != 0 {
            self.sweep_calculations(channel_enable);
        }
    }

    fn sweep_calculations(&mut self, channel_enable: &mut bool) -> u16 {
        let mut temp_shadow = (self.sweep_frequency_shadow >> self.sweep_shift);
        if self.sweep_negate {
            self.done_negate_calc = true;
            // Take the 2's complement value
            temp_shadow = !temp_shadow;
            temp_shadow = temp_shadow.wrapping_add(1);
        }
        //TODO: Check what desired behaviour is, some roms (e.g Crystal) have unchecked overflow here.
        let new_value = temp_shadow.wrapping_add(self.sweep_frequency_shadow);

        if new_value > 2047 {
            *channel_enable = false;
            self.sweep_enabled = false;
        }

        new_value
    }

    pub fn read_register(&self) -> u8 {
        (self.sweep_period << 4) | self.sweep_shift | if self.sweep_negate { 0x8 } else { 0 }
    }

    pub fn write_register(&mut self, value: u8, channel_enable: &mut bool) {
        let old_negate = self.sweep_negate;
        self.sweep_negate = test_bit(value, 3);
        // Exiting negate mode disables channel after having done a sweep calculation in negate mode.
        if old_negate && !self.sweep_negate && self.done_negate_calc {
            *channel_enable = false;
            self.done_negate_calc = false;
        }
        self.sweep_period = (value >> 4) & 0x7;
        self.sweep_shift = value & 0x7;
    }
}
