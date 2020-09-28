use crate::hardware::apu::channel_features::{EnvelopeFeature, SweepFeature, LengthFeature};

#[derive(Default, Debug)]
pub struct Voice1 {
    /// 0xFF14 TL-- -FFF  Trigger, Length enable, Frequency MSB
    pub nr14: u8,

    pub length: LengthFeature,
    pub envelope: EnvelopeFeature,

    // Sweep
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    // Internal Sweep
    sweep_enabled: bool,
    sweep_timer: u8,
    sweep_frequency_shadow: u16,

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

    pub fn output_volume(&self) -> u8 {
        self.output_volume
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

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

    pub fn read_register(&self, address: u16) -> u8 {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 => self.read_sweep_register(),
            0x11 => ((self._duty_select as u8) << 6) | self.length.read_register(),
            0x12 => self.envelope.read_register(),
            0x13 => 0xFF, // Can't read NR13
            0x14 => self.get_nr14(),
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address)
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 => self.write_sweep_register(value),
            0x11 => {
                self._duty_select = ((value & 0b1100_0000) >> 6) as usize;
                self.length.write_register(value);
            }
            0x12 => self.envelope.write_register(value),
            0x13 => self._frequency = (self._frequency & 0x0700) | value as u16,
            0x14 => {
                self.enabled = (value & 0x80) != 0;
                self.length.length_enable = (value & 0x4) == 0x4;
                self._frequency = (self._frequency & 0xFF) | (((value & 0x07) as u16) << 8);
                // TODO: Check if this occurs always, or only if the previous _triggered == false
                if self.enabled {
                    self.enable();
                }
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address)
        }
    }

    fn enable(&mut self) {
        log::warn!("Enable Call!");
        // Values taken from: https://gist.github.com/drhelius/3652407
        self.enabled = true;
        self.length.trigger();
        self._timer = (2048 - self._frequency) * 4;
        self.envelope.trigger();
        self.trigger_sweep();

        // Default wave form should be selected.
        self._duty_select = 0x2;
    }

    fn get_nr14(&self) -> u8 {
        let mut output = if self.enabled { 0x80 } else { 0x0 };
        output |= if self.length.length_enable { 0x40 } else { 0x0 };
        output |= (self._frequency >> 8) as u8;

        output
    }

    // --- Length ---

    pub fn tick_length(&mut self) {
        // TODO: Check if this is correct, as I'm pretty sure channels should continue ticking
        // even when disabled.
        self.length.tick(&mut self.enabled);
    }

    // --- SWEEP ---

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

    /// Follows the behaviour when a channel is triggered, specifically for the Sweep feature.
    pub fn trigger_sweep(&mut self) {
        self.sweep_frequency_shadow = self._frequency;
        self.sweep_timer = self.sweep_period; // Not sure if it's the period?
        self.sweep_enabled = self.sweep_period != 0 && self.sweep_shift != 0;
        // If sweep shift != 0, question is if sweep_enable is OR or AND, because docs are ambiguous.
        if self.sweep_enabled {
            self.sweep_calculations();
        }
    }

    pub fn read_sweep_register(&self) -> u8 {
        (self.sweep_period << 4) | self.sweep_shift | if self.sweep_negate { 0x8 } else { 0 }
    }

    pub fn write_sweep_register(&mut self, value: u8) {
        self.sweep_period = (value >> 4) & 0x7;
        self.sweep_negate = (value & 0x8) == 0x8;
        self.sweep_shift = value & 0x7;
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
