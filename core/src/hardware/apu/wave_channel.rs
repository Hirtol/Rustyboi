use num_integer::Integer;

use crate::hardware::apu::channel_features::LengthFeature;
use crate::hardware::apu::{no_length_tick_next_step, test_bit};
use crate::hardware::mmu::INVALID_READ;

/// Relevant for voice 3 for the DMG.
///
/// # Properties:
/// * Length Counter
#[derive(Default, Debug)]
pub struct WaveformChannel {
    pub length: LengthFeature,
    pub timer: u16,
    frequency: u16,
    timer_load_value: u16,
    trigger: bool,
    output_volume: u8,

    dac_power: bool,
    volume_load: u8,
    volume: u8,
    sample_buffer: [u8; 32],
    wave_ram: [u8; 16],
    sample_pointer: usize,
    #[cfg(feature = "apu-logging")]
    pub cycles_done: u64,
}

impl WaveformChannel {
    pub fn new() -> Self {
        WaveformChannel {
            // The DMG initialisation values, the game R-Type relies on these
            // CGB are different.
            sample_buffer: [
                0x8, 0x4, 0x4, 0x0, 0x4, 0x3, 0xA, 0xA, 0x2, 0xD, 0x7, 0x8, 0x9, 0x2, 0x3, 0xC, 0x6, 0x0, 0x5, 0x9,
                0x5, 0x9, 0xB, 0x0, 0x3, 0x4, 0xB, 0x8, 0x2, 0xE, 0xD, 0xA,
            ],
            // Purely for the initial tick_timer()
            timer_load_value: 4096,
            ..Default::default()
        }
    }

    /// Output a sample for this channel, returns `0` if the channel or DAC isn't enabled.
    #[inline(always)]
    pub fn output_volume(&self) -> u8 {
        self.output_volume * self.trigger as u8 * self.dac_power as u8
    }

    pub fn triggered(&self) -> bool {
        self.trigger
    }

    pub fn tick_timer(&mut self, cycles: u64) {
        let (mut to_generate, remainder) = (cycles / self.timer_load_value as u64, (cycles % self.timer_load_value as u64) as u16);

        while to_generate > 0 {
            self.timer_load_value = (2048 - self.frequency) * 2;
            self.tick_calculations();
            to_generate -= 1;
        }

        if remainder > self.timer {
            let to_subtract = remainder - self.timer;
            self.load_timer_values();
            self.tick_calculations();
            // We use recursion here since it can happen that the timer_load_value is actually less than to_subtract
            self.tick_timer(to_subtract as u64);
        } else {
            self.timer -= remainder;
        }
    }

    #[inline]
    fn load_timer_values(&mut self) {
        // The formula is taken from gbdev, I haven't done the period calculations myself.
        self.timer_load_value = (2048 - self.frequency) * 2;
        self.timer = self.timer_load_value;
    }

    #[inline]
    fn tick_calculations(&mut self) {
        // If we overflowed we might've lost some cycles, so we should make up for those.
        // Selects which sample we should select in our chosen duty cycle.
        self.sample_pointer = (self.sample_pointer + 1) % 32;

        self.update_sample();
        #[cfg(feature = "apu-logging")]
            {
                self.cycles_done += 1;
            }
    }

    #[inline]
    pub fn update_sample(&mut self) {
        self.output_volume = (self.sample_buffer[self.sample_pointer] >> self.volume);
    }

    pub fn tick_length(&mut self) {
        self.length.tick(&mut self.trigger);
    }

    pub fn read_register(&self, address: u16) -> u8 {
        // Expect the address to already have had an & 0xFF
        // The read values are taken from gbdev
        match address {
            0x1A => {
                if self.dac_power {
                    INVALID_READ
                } else {
                    0x7F
                }
            }
            0x1B => INVALID_READ,
            0x1C => 0x9F | self.volume_load,
            0x1D => INVALID_READ,
            0x1E => {
                if self.length.length_enable {
                    INVALID_READ
                } else {
                    0xBF
                }
            }
            0x30..=0x3F => {
                // If the wave channel is enabled, accessing any byte from $FF30-$FF3F is equivalent
                // to accessing the current byte selected by the waveform position.
                // Further, on the DMG accesses will only work in this manner if made within a
                // couple of clocks of the wave channel accessing wave RAM;
                // if made at any other time, reads return $FF and writes have no effect.
                if self.trigger {
                    self.wave_ram[self.sample_pointer / 2]
                } else {
                    self.wave_ram[(address - 0x30) as usize]
                }
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8, next_frame_sequencer_step: u8) {
        // Expect the address to already have had an & 0xFF
        match address {
            0x1A => {
                self.dac_power = test_bit(value, 7);
                if !self.dac_power {
                    self.trigger = false;
                }
            }
            0x1B => self.length.write_register_256(value),
            0x1C => self.set_volume_from_val(value),
            0x1D => {
                self.frequency = (self.frequency & 0x0700) | value as u16;
                // Say we have don't have the below 2 statements and this happens:
                // Timer load value: 32 and timer: 32
                // 16 cycles pass
                // Synchronise, timer gets to 16.
                // New Frequency gets set and would cause timer_load to be set to: 4096
                // 96 cycles pass
                // Synchronise, to_generate gets set to 3 (since 96/32 = 3),
                // but in truth we should only do one cycle since 96-16 = 80, which
                // is less than 4096 obviously.
                // Therefore, this timer load is necessary to ensure that doesn't happen in our catch up
                // cycling. Obviously, this is only relevant if the new load value is greater than
                // our existing one.
                // Do note that for passing test roms this case doesn't matter, since nothing tests
                // for this sort of behaviour (and in actual games it doesn't matter that much either)
                // but since it's so cheap we'll keep it here for the sake of accuracy.
                let temp_timer_load = (2048 - self.frequency) * 2;
                if  temp_timer_load > self.timer_load_value {
                    self.timer_load_value = temp_timer_load;
                }
            },
            0x1E => {
                let old_length_enable = self.length.length_enable;
                let no_l_next = no_length_tick_next_step(next_frame_sequencer_step);

                self.length.length_enable = test_bit(value, 6);
                self.frequency = (self.frequency & 0x00FF) | (((value & 0x07) as u16) << 8);
                // See comment in 0x1D branch
                let temp_timer_load = (2048 - self.frequency) * 2;
                if  temp_timer_load > self.timer_load_value {
                    self.timer_load_value = temp_timer_load;
                }

                if self.length.length_enable && !old_length_enable && no_l_next {
                    self.length.second_half_enable_tick(&mut self.trigger, old_length_enable);
                }

                if test_bit(value, 7) {
                    self.trigger(no_l_next);
                }
            }
            0x30..=0x3F => {
                #[cfg(feature = "apu-logging")]
                log::warn!("Writing current pointer: {} ({}) and wave ram: {:#X?}", self.sample_pointer, self.sample_pointer / 2, self.wave_ram);
                if self.trigger {
                    self.wave_ram[self.sample_pointer / 2] = value;
                } else {
                    let offset_address = ((address - 0x30) * 2) as usize;
                    self.sample_buffer[offset_address] = value >> 4;
                    self.sample_buffer[offset_address + 1] = value & 0xF;

                    self.wave_ram[offset_address / 2] = value;
                }
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    pub fn reset(&mut self) {
        self.length.length_enable = false;
        self.length.length_timer = 256;
        self.output_volume = 0;
        self.sample_pointer = 0;
        self.trigger = false;
        self.dac_power = false;
        self.volume_load = 0;
        self.volume = 0;
        self.timer_load_value = 4096;
        self.timer = self.timer_load_value;
        self.frequency = 0;
    }

    /// Should be called whenever the trigger bit in NR34 is written to.
    ///
    /// The values that are set are taken from [here](https://gist.github.com/drhelius/3652407)
    fn trigger(&mut self, next_step_no_length: bool) {
        self.trigger = true;
        self.length.trigger_256(next_step_no_length);
        self.timer_load_value = (2048 - self.frequency) * 2;
        self.timer = self.timer_load_value;
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
        };

        // Since the volume is different we should update our current sample.
        self.update_sample();
    }
}
