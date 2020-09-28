use crate::hardware::apu::channel_features::{EnvelopeFeature, SweepFeature, LengthFeature};

#[derive(Default, Debug)]
pub struct Voice1 {
    /// 0xFF14 TL-- -FFF  Trigger, Length enable, Frequency MSB
    pub nr14: u8,

    pub length: LengthFeature,
    pub envelope: EnvelopeFeature,
    pub sweep: SweepFeature,

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
            0x10 => self.sweep.read_register(),
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
            0x10 => self.sweep.write_register(value),
            0x11 => {
                self._duty_select = ((value & 0b1100_0000) >> 6) as usize;
                self.length.write_register(value);
            }
            0x12 => self.envelope.write_register(value),
            0x13 => self._frequency = (self._frequency & 0x0700) | value as u16,
            0x14 => {
                self.enabled = (value & 0x80) != 0;
                self.length.length_enable = (value & 0x40) == 0x40;
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
        self.sweep.trigger_sweep(&mut self.enabled, self._frequency);

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
        self.sweep.tick(&mut self.enabled, &mut self._frequency);
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
