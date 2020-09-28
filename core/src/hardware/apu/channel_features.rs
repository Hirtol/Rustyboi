#[derive(Default, Debug)]
pub struct EnvelopeFeature {
    pub volume: u8,
    volume_load: u8,
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

#[derive(Default, Debug)]
pub struct LengthFeature {
    pub length_enable: bool,
    length_load: u8,
    length_timer: u8,
}

impl LengthFeature {
    /// Ticks the length feature.
    ///
    /// # Returns
    /// * `true` - when the channel should stay enabled.
    /// * `false` - when the channel should be disabled.
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
    #[inline]
    pub fn trigger(&mut self) {
        //TODO: FIX THIS, AS CURRENTLY LENGTH DOESN'T WORK.
        if self.length_timer == 0 {
            self.length_timer = 64;
            // Not sure about this, but without it the Nintendo TRING gets cut off.
        }
    }

    #[inline]
    pub fn read_register(&self) -> u8 {
        self.length_load
    }

    #[inline]
    pub fn write_register(&mut self, value: u8) {
        self.length_load = value & 0x3F;
        // I think this is correct, not sure.
        self.length_timer = 64 - self.length_load;
    }

    /// Follows the behaviour for a wave channel, Length feature (256)
    #[inline]
    pub fn trigger_256(&mut self) {
        //TODO: VERIFY THIS
        if self.length_timer == 0 {
            self.length_timer = 255;
        }
    }

    #[inline]
    pub fn write_register_256(&mut self, value: u8) {
        self.length_load = value;
        // I think this is correct, not sure.
        self.length_timer = 255 - self.length_load;
    }

}




// Consider using this if we can figure out bindings.
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

}