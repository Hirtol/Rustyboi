use crate::emulator::EmulatorMode;
use crate::hardware::apu::channel_features::{EnvelopeFeature, LengthFeature};
use crate::hardware::apu::{no_length_tick_next_step, test_bit};
use crate::hardware::mmu::INVALID_READ;

/// Relevant for voice 4 for the DMG.
///
/// # Properties:
/// * Volume Envelope
/// * Length Feature (? Not listed in docs, but has a register for it)
#[derive(Debug, Default)]
pub struct NoiseChannel {
    pub length: LengthFeature,
    envelope: EnvelopeFeature,
    trigger: bool,
    output_volume: u8,
    timer: u16,
    timer_load_value: u16,
    // Noise Feature
    width_mode: bool,
    clock_shift: u8,
    divisor_code: u8,
    // 15 bit linear feedback shift register
    lfsr: u16,
}

impl NoiseChannel {
    /// Output a sample for this channel, returns `0` if the channel isn't enabled.
    pub fn output_volume(&self) -> u8 {
        self.output_volume * self.trigger as u8
    }

    pub fn triggered(&self) -> bool {
        self.trigger
    }

    pub fn tick_timer(&mut self, cycles: u64) {
        let (mut to_generate, remainder) = if self.timer_load_value != 0 {
            (cycles / self.timer_load_value as u64, (cycles % self.timer_load_value as u64) as u16)
        } else {
            (0, cycles as u16)
        };

        while to_generate > 0 {
            let bit_1_and_0_xor = (self.lfsr & 0x1) ^ ((self.lfsr & 0x2) >> 1);
            self.lfsr >>= 1;
            self.lfsr |= bit_1_and_0_xor << 14;

            if self.width_mode {
                self.lfsr = (self.lfsr & 0xFFBF) | bit_1_and_0_xor << 6;
            }

            self.output_volume = (((!self.lfsr) & 0x1) as u8) * self.envelope.volume;
            to_generate -= 1;
        }

        if remainder >= self.timer {
            let to_subtract = remainder - self.timer;
            // The formula is taken from gbdev, I haven't done the period calculations myself.
            self.timer_load_value = self.get_divisor_from_code() << self.clock_shift;
            self.timer = self.timer_load_value - to_subtract;
            let bit_1_and_0_xor = (self.lfsr & 0x1) ^ ((self.lfsr & 0x2) >> 1);
            // Shift LFSR right by 1
            self.lfsr >>= 1;

            // Set the high bit (bit 14) to the XOR operation of before. Always done
            self.lfsr |= bit_1_and_0_xor << 14;

            if self.width_mode {
                // Set bit 6 as well, resulting in a 7bit LFSR.
                // We need the AND here since the XOR result could be 0 as well, which would
                // need to be set.
                self.lfsr = (self.lfsr & 0xFFBF) | bit_1_and_0_xor << 6;
            }
            // The result is taken from the current bit 0, inverted
            // Not sure about the envelope multiplication, docs don't mention it but I assume it's there
            // for a reason.
            self.output_volume = (((!self.lfsr) & 0x1) as u8) * self.envelope.volume;
        } else {
            self.timer -= remainder;
        }
    }

    pub fn tick_length(&mut self) {
        self.length.tick(&mut self.trigger);
    }

    pub fn tick_envelope(&mut self) {
        self.envelope.tick();
    }

    pub fn read_register(&self, address: u16) -> u8 {
        // Expect the address to already have had an & 0xFF
        // The read values are taken from gbdev
        match address {
            0x1F => INVALID_READ,
            0x20 => INVALID_READ,
            0x21 => self.envelope.read_register(),
            0x22 => (self.clock_shift << 4) | if self.width_mode { 0x8 } else { 0x0 } | self.divisor_code,
            0x23 => {
                if self.length.length_enable {
                    INVALID_READ
                } else {
                    0xBF
                }
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8, next_frame_sequencer_step: u8) {
        // Expect the address to already have had an & 0xFF
        match address {
            0x1F => {}
            0x20 => self.length.write_register(value),
            0x21 => {
                self.envelope.write_register(value);
                // If the DAC is disabled by this write we also disable the channel
                if self.envelope.volume_load == 0 {
                    self.trigger = false;
                }
            }
            0x22 => {
                self.clock_shift = value >> 4;
                self.divisor_code = value & 0x7;
                self.width_mode = test_bit(value, 3)
            }
            0x23 => {
                let old_length_enable = self.length.length_enable;
                let no_l_next = no_length_tick_next_step(next_frame_sequencer_step);

                self.length.length_enable = test_bit(value, 6);

                if no_l_next {
                    self.length
                        .second_half_enable_tick(&mut self.trigger, old_length_enable);
                }

                if test_bit(value, 7) {
                    self.trigger(no_l_next);
                }
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    /// Should be called whenever the trigger bit in NR44 is written to.
    ///
    /// The values that are set are taken from [here](https://gist.github.com/drhelius/3652407)
    fn trigger(&mut self, next_step_no_length: bool) {
        self.trigger = true;
        self.length.trigger(next_step_no_length);
        //TODO: Set this to next_step_envelope
        self.envelope.trigger(false);
        self.timer = self.get_divisor_from_code() << self.clock_shift;
        // Top 15 bits all set to 1
        self.lfsr = 0x7FFF;
        // If the DAC doesn't have power we ignore this trigger.
        // Why the add mode is relevant eludes me, but the PanDocs specifically mention it so..
        if self.envelope.volume_load == 0 && !self.envelope.envelope_add_mode {
            self.trigger = false;
        }
    }

    pub fn reset(&mut self, mode: EmulatorMode) {
        self.length.length_enable = false;

        *self = if mode.is_cgb() {
            Self::default()
        } else {
            Self {
                length: self.length,
                ..Default::default()
            }
        }
    }

    fn get_divisor_from_code(&self) -> u16 {
        match self.divisor_code {
            0 => 8,
            1 => 16,
            2 => 32,
            3 => 48,
            4 => 64,
            5 => 80,
            6 => 96,
            7 => 112,
            _ => unreachable!("Invalid divisor code set for noise channel!"),
        }
    }
}
