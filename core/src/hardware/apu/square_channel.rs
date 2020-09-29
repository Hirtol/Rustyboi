use crate::hardware::apu::channel_features::{EnvelopeFeature, LengthFeature, SweepFeature};
use crate::hardware::apu::test_bit;

/// Relevant for voice 1 and 2 for the DMG.
/// This is a rather dirty implementation where voice 1 and 2 are merged, the latter
/// simply not having its sweep function called.
///
/// # Properties:
/// * Sweep (only voice 1)
/// * Volume Envelope
/// * Length Counter
#[derive(Default, Debug)]
pub struct SquareWaveChannel {
    length: LengthFeature,
    envelope: EnvelopeFeature,
    sweep: SweepFeature,
    trigger: bool,
    output_volume: u8,
    frequency: u16,
    timer: u16,
    // Relevant for wave table indexing
    wave_table_pointer: usize,
    duty_select: usize,
}

impl SquareWaveChannel {
    const SQUARE_WAVE_TABLE: [[u8; 8]; 4] = [
        [0, 0, 0, 0, 0, 0, 0, 1], // 12.5% Duty cycle square
        [1, 0, 0, 0, 0, 0, 0, 1], // 25%
        [1, 0, 0, 0, 0, 1, 1, 1], // 50%
        [0, 1, 1, 1, 1, 1, 1, 0], // 75%
    ];

    pub fn output_volume(&self) -> u8 {
        if self.trigger {
            self.output_volume
        } else {
            0
        }
    }

    pub fn triggered(&self) -> bool {
        self.trigger
    }

    pub fn tick_timer(&mut self) {
        let (new_val, overflowed) = self.timer.overflowing_sub(1);

        if new_val == 0 || overflowed {
            // I got this from Reddit, lord only knows why specifically 2048.
            self.timer = (2048 - self.frequency) * 4;
            // Selects which sample we should select in our chosen duty cycle.
            // Refer to SQUARE_WAVE_TABLE constant.
            self.wave_table_pointer = (self.wave_table_pointer + 1) % 8;
            self.output_volume = if Self::SQUARE_WAVE_TABLE[self.duty_select][self.wave_table_pointer] == 1 {
                self.envelope.volume
            } else {
                0
            };
        } else {
            self.timer = new_val;
        }
    }

    pub fn read_register(&self, address: u16) -> u8 {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 => 0x80 | self.sweep.read_register(),
            0x11 | 0x16 => 0x3F | ((self.duty_select as u8) << 6),
            0x12 | 0x17 => self.envelope.read_register(),
            0x13 | 0x18 => 0xFF, // Can't read NR13
            0x14 | 0x19 => 0xBF | if self.length.length_enable { 0x40 } else { 0x0 },
            0x15 => 0xFF, // The second square wave channel doesn't have a sweep feature.
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 | 0x15 => self.sweep.write_register(value),
            0x11 | 0x16 => {
                self.duty_select = ((value & 0b1100_0000) >> 6) as usize;
                self.length.write_register(value);
            }
            0x12 | 0x17 => {
                self.envelope.write_register(value);
                // If the DAC is disabled by this write we disable the channel
                if self.envelope.volume_load == 0 {
                    self.trigger = false;
                }
            },
            0x13 | 0x18 => self.frequency = (self.frequency & 0x0700) | value as u16,
            0x14 | 0x19 => {
                // This trigger can only be reset by internal counters, thus we only check to set it
                // if we haven't already triggered the channel
                if !self.trigger {
                    self.trigger = test_bit(value, 7);
                }
                self.length.length_enable = (value & 0x40) == 0x40;
                self.frequency = (self.frequency & 0xFF) | (((value & 0x07) as u16) << 8);

                if self.trigger {
                    self.trigger();
                }
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    /// Should be called whenever the trigger bit in NR14 is written to.
    ///
    /// The values that are set are taken from [here](https://gist.github.com/drhelius/3652407)
    fn trigger(&mut self) {
        self.length.trigger();
        self.timer = (2048 - self.frequency) * 4;
        self.envelope.trigger();
        self.sweep.trigger_sweep(&mut self.trigger, self.frequency);

        // Default wave form should be selected.
        self.duty_select = 0x2;
        // If the DAC doesn't have power we ignore this trigger.
        // Why the add mode is relevant eludes me, but the PanDocs specifically mention it so..
        if self.envelope.volume_load == 0 && !self.envelope.envelope_add_mode {
            self.trigger = false;
        }
    }

    pub fn tick_envelope(&mut self) {
        self.envelope.tick();
    }

    pub fn tick_length(&mut self) {
        self.length.tick(&mut self.trigger);
    }

    pub fn tick_sweep(&mut self) {
        self.sweep.tick(&mut self.trigger, &mut self.frequency);
    }
}
