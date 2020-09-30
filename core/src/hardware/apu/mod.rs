use crate::hardware::apu::noise_channel::NoiseChannel;
use crate::hardware::apu::square_channel::SquareWaveChannel;
use crate::hardware::apu::wave_channel::WaveformChannel;

mod channel_features;
mod noise_channel;
mod square_channel;
mod wave_channel;

// Currently chose for 44100/60 = 739 samples per frame to make it 'kinda' sync up.
// In all likelihood this will cause issues due to scheduling delays so this should go up probably.
pub const SAMPLE_SIZE_BUFFER: usize = 739;

pub const APU_MEM_START: u16 = 0xFF10;
pub const APU_MEM_END: u16 = 0xFF2F;
pub const WAVE_SAMPLE_START: u16 = 0xFF30;
pub const WAVE_SAMPLE_END: u16 = 0xFF3F;

pub struct APU {
    voice1: SquareWaveChannel,
    voice2: SquareWaveChannel,
    voice3: WaveformChannel,
    voice4: NoiseChannel,
    // The vins are unused by by games, but for the sake of accuracy tests will be kept here.
    vin_l_enable: bool,
    vin_r_enable: bool,
    left_volume: u8,
    right_volume: u8,
    // 0-3 will represent voice 1-4 enable respectively.
    left_channel_enable: [bool; 4],
    right_channel_enable: [bool; 4],
    global_sound_enable: bool,
    output_buffer: Vec<f32>,
    sampling_handler: u8,
    frame_sequencer: u16,
    frame_sequencer_step: u8,
}

impl APU {
    pub fn new() -> Self {
        APU {
            voice1: Default::default(),
            voice2: Default::default(),
            voice3: WaveformChannel::new(),
            voice4: Default::default(),
            vin_l_enable: false,
            vin_r_enable: false,
            left_volume: 7,
            right_volume: 7,
            left_channel_enable: [true; 4],
            right_channel_enable: [true, true, false, false],
            // Start the APU with 2 frames of audio buffered
            output_buffer: Vec::with_capacity(SAMPLE_SIZE_BUFFER * 2),
            frame_sequencer: 0,
            sampling_handler: 0,
            global_sound_enable: true,
            frame_sequencer_step: 0,
        }
    }

    pub fn tick(&mut self, mut delta_cycles: u64) {
        if !self.global_sound_enable {
            return;
        }
        // These values are purely personal preference, may even want to defer this to the emulator
        // consumer.
        let left_final_volume = self.left_volume as f32 / 6.0;
        let right_final_volume = self.right_volume as f32 / 6.0;

        while delta_cycles > 0 {
            self.voice1.tick_timer();
            self.voice2.tick_timer();
            self.voice3.tick_timer();
            self.voice4.tick_timer();

            self.frame_sequencer += 1;
            if self.frame_sequencer >= 8192 {
                // The frame sequencer component clocks at 512Hz apparently.
                // 4194304/512 = 8192
                self.frame_sequencer -= 8192;
                match self.frame_sequencer_step {
                    0 | 4 => self.tick_length(),
                    2 | 6 => {
                        self.tick_length();
                        self.tick_sweep();
                    }
                    7 => self.tick_envelop(),
                    _ => {}
                }
                self.frame_sequencer_step = (self.frame_sequencer_step + 1) % 8;
            }

            // This block is here such that we get ~44100 samples per second, otherwise we'd generate
            // far more than we could consume.
            // TODO: Add actual downsampling instead of the selective audio pick.
            // Refer to: https://www.reddit.com/r/EmuDev/comments/g5czyf/sound_emulation/
            self.sampling_handler += 1;
            if self.sampling_handler == 95 {
                // Close enough value such that we get one sample every ~1/44100
                self.sampling_handler -= 95;

                // Left Audio
                self.generate_audio(self.left_channel_enable, left_final_volume);
                // Right Audio
                self.generate_audio(self.right_channel_enable, right_final_volume);
            }

            delta_cycles -= 1;
        }
    }

    pub fn get_audio_buffer(&self) -> &[f32] {
        &self.output_buffer
    }

    pub fn clear_audio_buffer(&mut self) {
        self.output_buffer.clear();
    }

    pub fn read_register(&self, address: u16) -> u8 {
        let address = address & 0xFF;

        match address {
            0x10..=0x14 => self.voice1.read_register(address),
            0x15..=0x19 => self.voice2.read_register(address),
            0x1A..=0x1E => self.voice3.read_register(address),
            0x1F..=0x23 => self.voice4.read_register(address),
            // APU registers
            0x24 => {
                let mut output = 0;
                set_bit(&mut output, 7, self.vin_l_enable);
                set_bit(&mut output, 3, self.vin_r_enable);
                output | (self.left_volume << 4) | self.right_volume
            }
            0x25 => {
                let mut output = 0;
                for i in 0..4 {
                    set_bit(&mut output, i as u8, self.right_channel_enable[i]);
                }
                for i in 0..4 {
                    set_bit(&mut output, i as u8 + 4, self.left_channel_enable[i]);
                }
                output
            }
            0x26 => {
                let mut output = 0x70;
                set_bit(&mut output, 7, self.global_sound_enable);
                set_bit(&mut output, 3, self.voice4.triggered());
                set_bit(&mut output, 2, self.voice3.triggered());
                set_bit(&mut output, 1, self.voice2.triggered());
                set_bit(&mut output, 0, self.voice1.triggered());
                output
            },
            0x27..=0x2F => 0xFF, // Unused registers, always read 0xFF
            _ => panic!("Out of bound APU register read: {}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        let address = address & 0xFF;

        // It's not possible to access any registers beside 0x26 while the sound is disabled.
        if !self.global_sound_enable && address != 0x26 {
            log::warn!("Tried to write APU while inaccessible");
            return;
        }

        self.test_length_sequencer_edge_case(address, value);

        match address {
            0x10..=0x14 => self.voice1.write_register(address, value, self.frame_sequencer_step),
            0x15..=0x19 => self.voice2.write_register(address, value, self.frame_sequencer_step),
            0x1A..=0x1E => self.voice3.write_register(address, value, self.frame_sequencer_step),
            0x1F..=0x23 => self.voice4.write_register(address, value, self.frame_sequencer_step),
            0x24 => {
                self.vin_l_enable = test_bit(value, 7);
                self.vin_r_enable = test_bit(value, 3);
                self.right_volume = value & 0x07;
                self.left_volume = (value & 0x70) >> 4;
            }
            0x25 => {
                for i in 0..4 {
                    self.right_channel_enable[i] = test_bit(value, i as u8);
                }
                for i in 0..4 {
                    self.left_channel_enable[i] = test_bit(value, i as u8 + 4);
                }
            }
            0x26 => {
                self.global_sound_enable = test_bit(value, 7);
                self.reset();
            },
            0x27..=0x2F => {} // Writes to unused registers are silently ignored.
            _ => panic!("Attempt to write to an unknown audio register: 0xFF{:02X} with val: {}", address, value),
        }
    }

    pub fn read_wave_sample(&self, address: u16) -> u8 {
        let address = address & 0xFF;
        self.voice3.read_register(address)
    }

    pub fn write_wave_sample(&mut self, address: u16, value: u8) {
        let address = address & 0xFF;
        self.voice3.write_register(address, value, self.frame_sequencer_step)
    }

    fn generate_audio(&mut self, voice_enables: [bool; 4], final_volume: f32) {
        let mut result = 0f32;
        // Voice 1 (Square wave)
        if voice_enables[0] {
            //result += (self.voice1.output_volume() as f32 / 100.0) * final_volume;
        }
        // Voice 2 (Square wave)
        if voice_enables[1] {
            //result += (self.voice2.output_volume() as f32 / 100.0) * final_volume;
        }
        // Voice 3 (Wave)
        if voice_enables[2] {
            result += (self.voice3.output_volume() as f32 / 100.0) * final_volume;
        }
        // Voice 4 (Noise)
        if voice_enables[3] {
            //result += (self.voice4.output_volume() as f32 / 100.0) * final_volume
        }

        self.output_buffer.push(result);
    }

    fn tick_length(&mut self) {
        self.voice1.tick_length();
        self.voice2.tick_length();
        self.voice3.tick_length();
        self.voice4.tick_length();
    }

    fn tick_envelop(&mut self) {
        self.voice1.tick_envelope();
        self.voice2.tick_envelope();
        self.voice4.tick_envelope();
    }

    fn tick_sweep(&mut self) {
        self.voice1.tick_sweep();
    }

    fn reset(&mut self) {
        self.voice1 = SquareWaveChannel::default();
        self.voice2 = SquareWaveChannel::default();
        self.voice3.reset();
        self.voice4 = NoiseChannel::default();
        self.vin_l_enable = false;
        self.vin_r_enable = false;
        self.right_volume = 0;
        self.left_volume = 0;
        self.left_channel_enable = [false; 4];
        self.right_channel_enable = [false; 4]
    }

    /// Tests for a particular edge case (described within the method) when the address points
    /// to one of the NRx4 registers of the voices.
    fn test_length_sequencer_edge_case(&mut self, address: u16, value: u8) {
        // If we write to length when the next step in the frame sequencer DOESN't tick length
        // AND if the length counter was previously disabled and now enabled AND the length
        // counter isn't zero it is then decremented once.
        // Due to the fact that we increment frame_sequencer immediately we have to check for current_step + 1
        if [0x14, 0x19, 0x1E, 0x23].contains(&address)
            && [1, 3, 5, 7].contains(&self.frame_sequencer_step)
            && test_bit(value, 6) {
            log::warn!("Reached ticking Length!");
            // Have to clone or else we have a mut and immut reference ._.
            let length_feature = match address {
                0x14 => self.voice1.length.clone(),
                0x19 => self.voice2.length.clone(),
                0x1E => self.voice3.length.clone(),
                0x23 => self.voice4.length.clone(),
                _ => panic!("Invalid read address passed: 0xFF{:02X}", address),
            };

            if !length_feature.length_enable && length_feature.length_timer > 0 {
                //TODO: Think of a way to avoid the double match for the same thing.
                log::warn!("Ticking Length, before: {}", length_feature.length_timer);
                match address {
                    0x14 => {
                        // We have to preemptively set length_enable to true, or else the tick
                        // won't work. This is not an ideal architecture and I should redo this
                        // later.
                        self.voice1.length.length_enable = true;
                        self.voice1.tick_length()
                    },
                    0x19 => {
                        self.voice2.length.length_enable = true;
                        self.voice2.tick_length()
                    },
                    0x1E => {
                        self.voice3.length.length_enable = true;
                        self.voice3.tick_length()
                    },
                    0x23 => {
                        self.voice4.length.length_enable = true;
                        self.voice4.tick_length()
                    },
                    _ => panic!("Invalid read address passed: 0xFF{:02X}", address),
                };
                log::warn!("After: {}", self.voice1.length.length_timer);
            }
        }
    }
}

fn no_length_tick_next_step(next_frame_sequence_val: u8) -> bool {
    [1, 3, 5, 7].contains(&next_frame_sequence_val)
}

fn set_bit(output: &mut u8, bit: u8, set: bool) {
    if set {
        *output |= 1 << bit;
    }
}

fn test_bit(value: u8, bit: u8) -> bool {
    let mask = 1 << bit;
    (value & mask) == mask
}
