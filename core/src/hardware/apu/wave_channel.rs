use num_integer::Integer;

use crate::hardware::apu::channel_features::LengthFeature;
use crate::hardware::apu::test_bit;

/// Relevant for voice 3 for the DMG.
///
/// # Properties:
/// * Length Counter
#[derive(Default, Debug)]
pub struct WaveformChannel {
    pub length: LengthFeature,
    trigger: bool,
    output_volume: u8,
    frequency: u16,
    timer: u16,

    dac_power: bool,
    volume_load: u8,
    volume: u8,
    sample_buffer: [u8; 32],
    sample_pointer: usize,
}

impl WaveformChannel {
    pub fn new() -> Self {
        WaveformChannel {
            // The DMG initialisation values, the game R-Type relies on these
            // CGB are different.
            sample_buffer: [0x8, 0x4, 0x4, 0x0, 0x4, 0x3, 0xA, 0xA, 0x2, 0xD, 0x7, 0x8, 0x9, 0x2, 0x3, 0xC, 0x6, 0x0, 0x5, 0x9, 0x5, 0x9, 0xB, 0x0, 0x3, 0x4, 0xB, 0x8, 0x2, 0xE, 0xD, 0xA],
            ..Default::default()
        }
    }

    pub fn output_volume(&self) -> u8 {
        if self.trigger && self.dac_power {
            self.output_volume
        } else {
            0
        }
    }

    pub fn triggered(&self) -> bool {
        self.trigger
    }

    pub fn tick_timer(&mut self) {
        //TODO: Fix
        let (new_val, overflowed) = self.timer.overflowing_sub(1);

        if new_val == 0 || overflowed {
            // The formula is taken from gbdev, I haven't done the period calculations myself.
            self.timer = (2048 - self.frequency) * 2;
            // Selects which sample we should select in our chosen duty cycle.
            self.sample_pointer = (self.sample_pointer + 1) % 32;

            self.output_volume = (self.sample_buffer[self.sample_pointer] >> self.volume);
        } else {
            self.timer = new_val;
        }
    }

    pub fn tick_length(&mut self) {
        self.length.tick(&mut self.trigger);
    }

    pub fn read_register(&self, address: u16) -> u8 {
        // Expect the address to already have had an & 0xFF
        // The read values are taken from gbdev
        match address {
            0x1A => 0x7F | if self.dac_power { 0x80 } else { 0 },
            0x1B => 0xFF,
            0x1C => 0x9F | self.volume_load,
            0x1D => 0xFF,
            0x1E => 0xBF | if self.length.length_enable { 0x40 } else { 0x0 },
            0x30..=0x3F => {
                let offset_address = ((address - 0x30) * 2) as usize;
                (self.sample_buffer[offset_address] << 4) | self.sample_buffer[offset_address + 1]
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        // Expect the address to already have had an & 0xFF
        match address {
            0x1A => self.dac_power = (value & 0x80) == 0x80,
            0x1B => self.length.write_register_256(value),
            0x1C => {
                self.set_volume_from_val(value);
                // If we're at 0% volume we've practically disabled the DAC, and thus should
                // disable the channel as well.
                if self.volume_load == 0 {
                    self.trigger = false;
                }
            },
            0x1D => self.frequency = (self.frequency & 0x0700) | value as u16,
            0x1E => {
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
            0x30..=0x3F => {
                log::warn!("Writing Samples");
                let offset_address = ((address - 0x30) * 2) as usize;
                self.sample_buffer[offset_address] = value >> 4;
                self.sample_buffer[offset_address + 1] = value & 0xF;
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    pub fn reset(&mut self) {
        self.length.length_enable = false;
        self.length.length_timer = 0;
        self.sample_pointer = 0;
        self.trigger = false;
        self.dac_power = false;
        self.volume_load = 0;
        self.volume = 0;
        self.timer = 0;
        self.frequency = 0;
    }

    /// Should be called whenever the trigger bit in NR34 is written to.
    ///
    /// The values that are set are taken from [here](https://gist.github.com/drhelius/3652407)
    fn trigger(&mut self) {
        self.length.trigger_256();
        self.timer = (2048 - self.frequency) * 2;
        self.sample_pointer = 0;
        self.set_volume_from_val(self.volume_load);
        // If the DAC doesn't have power we ignore this trigger.
        if !self.dac_power {
            self.trigger = false;
        }
    }

    fn set_volume_from_val(&mut self, value: u8) {
        self.volume_load = value & 0x60;
        // We'll shift right (thus divide by 2) by these amounts.
        self.volume = match self.volume_load {
            0b0_00_0_0000 => 4, // 0% volume
            0b0_01_0_0000 => 0, // 100% volume
            0b0_10_0_0000 => 1, // 50% volume
            0b0_11_0_0000 => 2, // 75% volume
            _ => panic!("Received invalid entry in set_volume!"),
        }
    }
}
