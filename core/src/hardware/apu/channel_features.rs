#[derive(Default, Debug)]
pub struct EnvelopeFeature {
    pub volume: u8,
    pub volume_load: u8,
    pub envelope_add_mode: bool,
    envelope_enabled: bool,
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
        self.volume = self.volume_load;
    }

    pub fn read_register(&self) -> u8 {
        (self.volume_load << 4) | self.envelope_period_load_value | if self.envelope_add_mode { 0x8 } else { 0 }
    }

    pub fn write_register(&mut self, value: u8) {
        self.volume_load = (value & 0xF0) >> 4;
        // Not sure if to reload this value.
        self.volume = self.volume_load;
        self.envelope_add_mode = (value & 0x8) == 0x8;
        self.envelope_period_load_value = value & 0x7;
        // Does this immediately load?
        self.envelope_period = self.envelope_period_load_value;
    }
}

#[derive(Default, Debug, Clone)]
pub struct LengthFeature {
    pub length_enable: bool,
    pub length_timer: u16,
    length_load: u8,
}

impl LengthFeature {

    pub fn peculiar_tick(&mut self, channel_enable: &mut bool, old_enable: bool) {
        if !old_enable {
            self.tick(channel_enable);
        }
    }
    /// Ticks the length feature.
    ///
    /// # Manipulations
    /// * `channel_enable` - Is reset when the length timer reaches 0, otherwise untouched.
    pub fn tick(&mut self, channel_enable: &mut bool) {
        // Not sure whether to have length_load become a separate timer, and use the
        // length_load field as a load_value instead like we've done with envelop/sweep.
        if self.length_enable && self.length_timer > 0 {
            self.length_timer -= 1;

            if self.length_timer == 0 {
                log::warn!("OFF");
                *channel_enable = false;
            }
        }
    }

    /// Follows the behaviour when a channel is triggered for the Length feature (64)
    pub fn trigger(&mut self, next_step_no_length: bool) {
        if self.length_timer == 0 {
            if next_step_no_length && self.length_enable {
                self.length_timer = 63;
            } else {
                self.length_timer = 64;
            }
        }
    }

    pub fn read_register(&self) -> u8 {
        self.length_load
    }

    pub fn write_register(&mut self, value: u8) {
        self.length_load = value & 0x3F;
        // I think this is correct, not sure.
        self.length_timer = 64 - self.length_load as u16;
    }

    /// Follows the behaviour for a wave channel, Length feature (256)
    pub fn trigger_256(&mut self, next_step_no_length: bool) {
        if self.length_timer == 0 {
            if next_step_no_length && self.length_enable {
                self.length_timer = 255;
            } else {
                self.length_timer = 256;
            }
        }
    }

    pub fn write_register_256(&mut self, value: u8) {
        self.length_load = value;
        // I think this is correct, not sure.
        self.length_timer = 256 - self.length_load as u16;
    }
}

#[derive(Default, Debug)]
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
    /// Ticks the sweep feature.
    /// Expects the channel enable and frequency
    ///
    /// # Manipulations
    /// * `channel_enable` - Is reset when the sum of the
    /// shifted shadow frequency + shadow frequency is >= 2048, otherwise untouched.
    /// * `channel_frequency` - Is set to a new value when the tick function has a shadow frequency
    /// lower than 2048.
    pub fn tick(&mut self, channel_enable: &mut bool, channel_frequency: &mut u16) {
        if self.sweep_enabled && self.sweep_period != 0 {
            let temp_freq = self.sweep_calculations(channel_enable);
            // Duplicate overflow check, but this gets called, at most 128 times per second so, eh.
            if temp_freq < 2048 && self.sweep_shift != 0 {
                self.sweep_frequency_shadow = temp_freq;
                *channel_frequency = temp_freq;
                self.sweep_calculations(channel_enable);
            }
        }
    }

    /// Follows the behaviour when a channel is triggered, specifically for the Sweep feature.
    pub fn trigger_sweep(&mut self, channel_enable: &mut bool, frequency: u16) {
        self.sweep_frequency_shadow = frequency;
        //TODO: Remove as it seems unnecessary?
        self.sweep_timer = self.sweep_period; // Not sure if it's the period?
        self.sweep_enabled = self.sweep_period != 0 && self.sweep_shift != 0;
        // If sweep shift != 0, question is if sweep_enable is OR or AND, because docs are ambiguous.
        if self.sweep_enabled {
            self.sweep_calculations(channel_enable);
        }
    }

    fn sweep_calculations(&mut self, channel_enable: &mut bool) -> u16 {
        let mut temp_shadow = (self.sweep_frequency_shadow >> self.sweep_shift);
        if self.sweep_negate {
            // Not sure if we should take 2's complement here, TODO: Verify.
            temp_shadow = !temp_shadow;
        }
        temp_shadow += self.sweep_frequency_shadow;

        if temp_shadow > 2047 {
            *channel_enable = false;
            self.sweep_enabled = false;
        }

        temp_shadow
    }

    pub fn read_register(&self) -> u8 {
        (self.sweep_period << 4) | self.sweep_shift | if self.sweep_negate { 0x8 } else { 0 }
    }

    pub fn write_register(&mut self, value: u8) {
        self.sweep_period = (value >> 4) & 0x7;
        self.sweep_negate = (value & 0x8) == 0x8;
        self.sweep_shift = value & 0x7;
    }
}
