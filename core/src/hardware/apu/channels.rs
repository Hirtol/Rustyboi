use crate::hardware::apu::channel_features::EnvelopeFeature;

pub trait AudioVoice {}


#[derive(Default, Debug)]
pub struct Voice1 {
    /// 0xFF10 -PPP NSSS  Sweep period, negate, shift
    pub nr10: u8,
    /// 0xFF11 DDLL LLLL  Duty, Length load (64-L)
    pub nr11: u8,
    /// 0xFF12 VVVV APPP  Starting volume, Envelope add mode, period
    pub nr12: u8,
    /// 0xFF13 FFFF FFFF  Frequency LSB
    /// Write only.
    pub nr13: u8,
    /// 0xFF14 TL-- -FFF  Trigger, Length enable, Frequency MSB
    pub nr14: u8,
    // Sweep
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    // Internal Sweep
    sweep_enabled: bool,
    sweep_timer: u8,
    sweep_frequency_shadow: u16,

    // Length
    length_load: u8,
    length_enable: bool,
    length_timer: u8,

    pub envelope: EnvelopeFeature,

    // Timer stuff
    _frequency: u16,
    _timer: u16,

    enabled: bool,
    // Maybe use if we do the while loops inside the APU instead of channels, then
    // we wouldn't need a sample buffer (how about down sampling?).
    output_volume: u8,
    // Relevant for wave table indexing
    _wave_table_pointer: usize,
    _duty_select: usize,
}

impl Voice1 {
    const SQUARE_WAVE_TABLE: [[u8; 8]; 4] = [
        [0, 0, 0, 0, 0, 0, 0, 1], // 12.5% Duty cycle square
        [1, 0, 0, 0, 0, 0, 0, 1], // 25%
        [1, 0, 0, 0, 0, 1, 1, 1], // 50%
        [0, 1, 1, 1, 1, 1, 1, 0]  // 75%
    ];

    pub fn tick_timer(&mut self) {
        let (new_val, overflowed) = self._timer.overflowing_sub(1);

        if overflowed {
            // I got this from Reddit, lord only knows why specifically 2048.
            self._timer = (2048 - self._frequency) * 4;
            // Selects which sample we should select in our chosen duty cycle.
            // Refer to SQUARE_WAVE_TABLE constant.
            self._wave_table_pointer = (self._wave_table_pointer + 1) % 8;
        } else {
            self._timer = new_val;
        }
        //TODO: Insert && self.enabled once we figure out why the early cutoff
        self.output_volume = if Self::SQUARE_WAVE_TABLE[self._duty_select][self._wave_table_pointer] == 1 && self.enabled {
            self.envelope.volume
        } else {
            0
        };
    }

    pub fn tick_length(&mut self) {
        // Not sure whether to have length_load become a separate timer, and use the
        // length_load field as a load_value instead like we've done with envelop/sweep.
        if self.length_enable && self.length_timer > 0 {
            self.length_timer -= 1;

            if self.length_timer == 0 {
                log::warn!("OFF");
                self.enabled = false;
            }
        }
    }

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

    pub fn read_register(&self, address: u16) -> u8 {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 => self.nr10,
            0x11 => self.nr11,
            0x12 => self.envelope.read_register(),
            0x13 => 0xFF, // Can't read NR13
            0x14 => self.nr14,
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address)
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 => {
                self.nr10 = value;
                self.sweep_period = (value >> 4) & 0x7;
                self.sweep_negate = (value & 0x8) == 0x8;
                self.sweep_shift = value & 0x7;
            }
            0x11 => {
                self.nr11 = value;
                self._duty_select = ((value & 0b1100_0000) >> 6) as usize;
                self.length_load = value & 0x3F;
                // I think this is correct, not sure.
                self.length_timer = 64 - self.length_load;
            }
            0x12 => self.envelope.write_register(value),
            0x13 => {
                self.nr13 = value;
                self._frequency = (self._frequency & 0x0700) | value as u16;
            }
            0x14 => {
                self.nr14 = value;
                self.enabled = (value & 0x80) != 0;
                self.length_enable = (value & 0x4) == 0x4;
                self._frequency = (self._frequency & 0xFF) | (((value & 0x07) as u16) << 8);
                // TODO: Check if this occurs always, or only if the previous _triggered == false
                if self.enabled {
                    self.enable();
                }
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address)
        }
    }

    pub fn output_volume(&self) -> u8 {
        self.output_volume
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    fn enable(&mut self) {
        // Values taken from: https://gist.github.com/drhelius/3652407
        self.enabled = true;
        self.envelope.trigger();
        self._duty_select = 0x2;
        //TODO: FIX THIS, AS CURRENTLY LENGTH DOESN'T WORK.
        if self.length_load == 0 {
            self.length_load = 64;
            // Not sure about this, but without it the Nintendo TRING gets cut off.
            self.length_timer = 0;
        }

        self._timer = (2048 - self._frequency) * 4;

        self.sweep_frequency_shadow = self._frequency;
        self.sweep_timer = self.sweep_period; // Not sure if it's the period?
        self.sweep_enabled = self.sweep_period != 0 && self.sweep_shift != 0;
        // If sweep shift != 0, question is if sweep_enable is OR or AND, because docs are ambiguous.
        if self.sweep_enabled {
            self.sweep_calculations();
        }
    }

    fn sweep_calculations(&mut self) -> u16 {
        let mut temp_shadow = (self.sweep_frequency_shadow >> self.sweep_shift);
        if self.sweep_negate {
            // Not sure if we should take 2's complement here, TODO: Verify.
            temp_shadow = !temp_shadow;
        }
        temp_shadow += self.sweep_frequency_shadow;

        if temp_shadow > 2047 {
            self.enabled = false;
            self.sweep_enabled = false;
        }

        temp_shadow
    }
}

/// Relevant for voice 1 and 2 for the DMG.
///
/// # Properties:
/// * Sweep (only voice 1)
/// * Volume Envelope
/// * Length Counter
pub struct SquareWaveChannel {
    has_sweep: bool,
}

/// Relevant for voice 3 for the DMG.
///
/// # Properties:
/// * Length Counter
pub struct WaveformChannel {}

/// Relevant for voice 4 for the DMG.
///
/// # Properties:
/// * Volume Envelope
pub struct NoiseChannel {}
