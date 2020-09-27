pub trait AudioVoice {}


#[derive(Default, Debug, Clone)]
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
    // Timer stuff
    _frequency: u16,
    _timer: u16,

    _triggered: bool,
    // Maybe use if we do the while loops inside the APU instead of channels, then
    // we wouldn't need a sample buffer (how about down sampling?).
    volume: u8,
    _volume_true: u8,
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
            self._timer = (2048 - self.get_frequency()) * 4;
            // Selects which sample we should select in our chosen duty cycle.
            // Refer to SQUARE_WAVE_TABLE constant.
            self._wave_table_pointer = (self._wave_table_pointer + 1) % 8;
        } else {
            self._timer = new_val;
        }

        self._volume_true = if Self::SQUARE_WAVE_TABLE[self._duty_select][self._wave_table_pointer] == 1 {
            self.volume
        } else {
            0
        };
    }

    pub fn read_register(&self, address: u16) -> u8 {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 => self.nr10,
            0x11 => self.nr11,
            0x12 => self.nr12,
            0x13 => 0xFF, // Can't read NR13
            0x14 => self.nr14,
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address)
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 => self.nr10 = value,
            0x11 => {
                self.nr11 = value;
                self._duty_select = ((value & 0b1100_0000) >> 6) as usize;
            },
            0x12 => {
                self.nr12 = value;
                self.volume = (value & 0xF0) >> 4;
            },
            0x13 => {
                self.nr13 = value;
                self._frequency = (self._frequency & 0x0700) | value as u16;
            },
            0x14 => {
                self.nr14 = value;
                self._triggered = (value & 0x80) != 0;
                if self._triggered {
                    self.enable();
                }
            },
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address)
        }
    }

    pub fn output_volume(&self) -> u8 {
        self._volume_true
    }

    pub fn enable(&mut self) {
        self.write_register(0x10, 0x80);
        self.write_register(0x11, 0xBF);
        self.write_register(0x12, 0xF3);
        // nr13 is purposefully skipped. Refer to:
        // https://github.com/AntonioND/giibiiadvance/blob/master/docs/other_docs/GBSOUND.txt
        self.write_register(0x14, 0xBF);
        self._duty_select = 0x2;
        self.sampling_handler = 95;

    }

    fn get_frequency(&self) -> u16 {
        (((self.nr14 & 0x07) as u16) << 8) | self.nr13 as u16
    }

    fn set_frequency(&mut self, new_value: u16) {
        self.nr13 = (new_value & 0x00FF) as u8;
        self.nr14 = ((new_value >> 8) & 0x07) as u8;
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

#[cfg(test)]
mod tests {
    use crate::hardware::apu::channels::Voice1;

    #[test]
    fn test_voice_frequency() {
        let mut voice1 = Voice1::default();
        assert_eq!(voice1.get_frequency(), 0);
        voice1.set_frequency(2000);
        assert_eq!(voice1.get_frequency(), 2000);
        voice1.set_frequency(248);
        assert_eq!(voice1.get_frequency(), 248);
    }
}
