use crate::hardware::apu::channel_features::{EnvelopeFeature, LengthFeature};
use crate::hardware::apu::test_bit;

/// Relevant for voice 4 for the DMG.
///
/// # Properties:
/// * Volume Envelope
/// * Length Feature (? Not listed in docs, but has a register for it)
#[derive(Debug, Default)]
pub struct NoiseChannel {
    envelope: EnvelopeFeature,
    length: LengthFeature,
    trigger: bool,
    output_volume: u8,
    timer: u16,
    // Noise Feature
    width_mode: bool,
    clock_shift: u8,
    divisor_code: u8,
    // 15 bit linear feedback shift register
    lfsr: u16,
}

impl NoiseChannel {
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
            self.timer = self.get_divisor_from_code() << self.clock_shift;
            // Selects which sample we should select in our chosen duty cycle.
            let bit_1_and_0_xor = (self.lfsr & 0x1) ^ ((self.lfsr & 0x2) >> 1);
            // Shift LFSR right by 1
            self.lfsr >>= 1;
            // Set the high bit (bit 14) to the XOR operation of before. Always done
            self.lfsr |= bit_1_and_0_xor << 14;

            if self.width_mode {
                // Set bit 6 as well, resulting in a 7bit LFSR.
                self.lfsr |= bit_1_and_0_xor << 6;
            }
            // The result is taken from the current bit 0, inverted
            // Not sure about the envelope multiplication, docs don't mention it but I assume it's there
            // for a reason.
            self.output_volume = (((!self.lfsr) & 0x1) as u8) * self.envelope.volume;
        } else {
            self.timer = new_val;
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
            0x1F => 0xFF,
            0x20 => 0xFF,
            0x21 => self.envelope.read_register(),
            0x22 => (self.clock_shift << 4) | if self.width_mode {0x8} else {0x0} | self.divisor_code,
            0x23 => 0xBF | if self.length.length_enable { 0x40 } else { 0x0 },
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        // Expect the address to already have had an & 0xFF
        match address {
            0x1F => {},
            0x20 => self.length.write_register(value),
            0x21 => {
                self.envelope.write_register(value);
                // If the DAC is disabled by this write we also disable the channel
                if self.envelope.volume_load == 0 {
                    self.trigger = false;
                }
            },
            0x22 => {
                self.clock_shift = value >> 4;
                self.divisor_code = value & 0x7;
                self.width_mode = test_bit(value, 3)
            },
            0x23 => {
                // This trigger can only be reset by internal counters, thus we only check to set it
                // if we haven't already triggered the channel
                if !self.trigger {
                    self.trigger = test_bit(value, 7);
                }
                self.length.length_enable = test_bit(value, 6);
                if self.trigger {
                    self.trigger();
                }
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    /// Should be called whenever the trigger bit in NR44 is written to.
    ///
    /// The values that are set are taken from [here](https://gist.github.com/drhelius/3652407)
    fn trigger(&mut self) {

        self.length.trigger();
        self.envelope.trigger();
        self.timer = self.get_divisor_from_code() << self.clock_shift;
        // Top 15 bits all set to 1
        self.lfsr = 0x7FFF;
        // If the DAC doesn't have power we ignore this trigger.
        // Why the add mode is relevant eludes me, but the PanDocs specifically mention it so..
        if self.envelope.volume_load == 0 && !self.envelope.envelope_add_mode {
            self.trigger = false;
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
            _ => panic!("Invalid divisor code set for noise channel!"),
        }
    }
}

