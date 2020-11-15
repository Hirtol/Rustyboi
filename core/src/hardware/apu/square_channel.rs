use crate::emulator::EmulatorMode;
use crate::hardware::apu::channel_features::{EnvelopeFeature, LengthFeature, SweepFeature};
use crate::hardware::apu::{no_length_tick_next_step, test_bit};
use crate::hardware::mmu::INVALID_READ;

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
    pub length: LengthFeature,
    envelope: EnvelopeFeature,
    sweep: SweepFeature,
    trigger: bool,
    output_volume: u8,
    frequency: u16,
    timer: u16,
    timer_load_value: u16,
    // Relevant for wave table indexing
    wave_table_index: usize,
    duty_select: usize,
}

impl SquareWaveChannel {
    const SQUARE_WAVE_TABLE: [[u8; 8]; 4] = [
        [0, 0, 0, 0, 0, 0, 0, 1], // 12.5% Duty cycle square
        [1, 0, 0, 0, 0, 0, 0, 1], // 25%
        [1, 0, 0, 0, 0, 1, 1, 1], // 50%
        [0, 1, 1, 1, 1, 1, 1, 0], // 75%
    ];

    pub fn new() -> SquareWaveChannel {
        SquareWaveChannel {
            // Purely for the initial tick_timer()
            timer_load_value: 8192,
            .. Default::default()
        }
    }

    /// Output a sample for this channel, returns `0` if the channel isn't enabled.
    pub fn output_volume(&self) -> u8 {
        self.output_volume * self.trigger as u8
    }

    pub fn triggered(&self) -> bool {
        self.trigger
    }

    pub fn tick_timer(&mut self, cycles: u64) {
        let (mut to_generate, remainder) = (cycles / self.timer_load_value as u64, (cycles % self.timer_load_value as u64) as u16);

        while to_generate > 0 {
            self.timer_load_value = (2048 - self.frequency) * 4;
            self.tick_calculations();
            to_generate -= 1;
        }

        if remainder > self.timer {
            let to_subtract = remainder - self.timer;
            self.load_timer_values();
            self.tick_timer(to_subtract as u64);
            self.tick_calculations();
        } else {
            self.timer -= remainder;
        }
    }

    #[inline]
    fn load_timer_values(&mut self) {
        // I got this from Reddit, lord only knows why specifically 2048.
        self.timer_load_value = (2048 - self.frequency) * 4;
        self.timer = self.timer_load_value;
    }

    #[inline]
    fn tick_calculations(&mut self) {
        // Selects which sample we should select in our chosen duty cycle.
        // Refer to SQUARE_WAVE_TABLE constant.
        self.wave_table_index = (self.wave_table_index + 1) % 8;
        // Could move this to the actual output_volume() function?
        self.output_volume =
            self.envelope.volume * Self::SQUARE_WAVE_TABLE[self.duty_select][self.wave_table_index];
    }

    pub fn read_register(&self, address: u16) -> u8 {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 => 0x80 | self.sweep.read_register(),
            0x11 | 0x16 => 0x3F | ((self.duty_select as u8) << 6),
            0x12 | 0x17 => self.envelope.read_register(),
            0x13 | 0x18 => INVALID_READ, // Can't read NR13
            0x14 | 0x19 => 0xBF | if self.length.length_enable { 0x40 } else { 0x0 },
            0x15 => INVALID_READ, // The second square wave channel doesn't have a sweep feature.
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8, next_frame_sequencer_step: u8) {
        // Expect the address to already have had an & 0xFF
        match address {
            0x10 | 0x15 => self.sweep.write_register(value, &mut self.trigger),
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
            }
            0x13 | 0x18 => {
                self.frequency = (self.frequency & 0x0700) | value as u16;
                // See wave channel write_register 0x1D for explanation
                let temp_timer_load = (2048 - self.frequency) * 2;
                if  temp_timer_load > self.timer_load_value {
                    self.timer_load_value = temp_timer_load;
                }
            },
            0x14 | 0x19 => {
                let old_length_enable = self.length.length_enable;
                let no_l_next = no_length_tick_next_step(next_frame_sequencer_step);

                self.length.length_enable = test_bit(value, 6);
                self.frequency = (self.frequency & 0xFF) | (((value & 0x07) as u16) << 8);

                // See wave channel write_register 0x1D for explanation
                let temp_timer_load = (2048 - self.frequency) * 2;
                if  temp_timer_load > self.timer_load_value {
                    self.timer_load_value = temp_timer_load;
                }

                if no_l_next {
                    self.length
                        .second_half_enable_tick(&mut self.trigger, old_length_enable);
                }

                // We specifically only trigger if the current write value is setting the trigger bit.
                if test_bit(value, 7) {
                    self.trigger(no_l_next);
                }
            }
            _ => panic!("Invalid Voice1 register read: 0xFF{:02X}", address),
        }
    }

    /// Should be called whenever the trigger bit in NR14 is written to.
    ///
    /// The values that are set are taken from [here](https://gist.github.com/drhelius/3652407)
    fn trigger(&mut self, next_step_no_length: bool) {
        self.trigger = true;
        self.length.trigger(next_step_no_length);
        //TODO: Set this to next_step_envelope
        self.envelope.trigger(false);
        self.timer_load_value = (2048 - self.frequency) * 4;
        self.timer = self.timer_load_value;
        self.sweep.trigger_sweep(&mut self.trigger, self.frequency);

        // Default wave form should be selected.
        self.duty_select = 0x2;
        // If the DAC doesn't have power we ignore this trigger.
        // Why the add mode is relevant eludes me, but the PanDocs specifically mention it so..
        if self.envelope.volume_load == 0 && !self.envelope.envelope_add_mode {
            self.trigger = false;
        }
    }

    pub fn reset(&mut self, mode: EmulatorMode) {
        self.length.length_enable = false;

        *self = if mode.is_cgb() {
            Self {
                timer_load_value: 8192,
                ..Default::default()
            }
        } else {
            Self {
                length: self.length,
                timer_load_value: 8192,
                ..Default::default()
            }
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
