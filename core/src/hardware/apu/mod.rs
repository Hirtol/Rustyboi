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
pub const APU_MEM_END: u16 = 0xFF3F;

pub struct APU {
    voice1: SquareWaveChannel,
    voice2: SquareWaveChannel,
    voice3: WaveformChannel,
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
        // It's not possible to access any registers beside 0x26 while the sound is disabled.
        if !self.global_sound_enable && address != 0x26 && !(0x30..=0x3F).contains(&address) {
            log::warn!("Tried to read APU while inaccessible");
            return 0xFF;
        }

        match address {
            0x10..=0x14 => self.voice1.read_register(address),
            0x15..=0x19 => self.voice2.read_register(address),
            0x1A..=0x1E | 0x30..=0x3F => self.voice3.read_register(address),
            // APU registers
            0x24 => self.right_volume | (self.left_volume << 4),
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
                //TODO: These three voices enable flags.
                set_bit(&mut output, 3, true);
                set_bit(&mut output, 2, self.voice3.enabled());
                set_bit(&mut output, 1, self.voice2.enabled());
                set_bit(&mut output, 0, self.voice1.enabled());
                output
            }
            //TODO: Once all voices are implemented bring back panic.
            _ => 0xFF, //panic!("Attempt to read an unknown audio register: 0xFF{:02X}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        let address = address & 0xFF;

        // It's not possible to access any registers beside 0x26 while the sound is disabled.
        if !self.global_sound_enable && address != 0x26 && !(0x30..=0x3F).contains(&address) {
            log::warn!("Tried to write APU while inaccessible");
            return;
        }

        match address {
            0x10..=0x14 => self.voice1.write_register(address, value),
            0x15..=0x19 => self.voice2.write_register(address, value),
            0x1A..=0x1E | 0x30..=0x3F => self.voice3.write_register(address, value),
            0x24 => {
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
                self.global_sound_enable = (value & 0b1000_0000) == 0b1000_0000;
                self.voice1 = SquareWaveChannel::default();
                self.voice2 = SquareWaveChannel::default();
                self.voice3.reset();
                //TODO: Reset all voices.
            },
            //TODO: Once all voices are implemented bring back panic.
            _ => {} //panic!("Attempt to write to an unknown audio register: 0xFF{:02X} with val: {}", address, value),
        }
    }

    fn generate_audio(&mut self, voice_enables: [bool; 4], final_volume: f32) {
        let mut result = 0f32;
        // Voice 1 (Square wave)
        if voice_enables[0] {
            result += (self.voice1.output_volume() as f32 / 100.0) * final_volume;
        }
        // Voice 2 (Square wave)
        if voice_enables[1] {
            result += (self.voice2.output_volume() as f32 / 100.0) * final_volume;
        }
        // Voice 3 (Wave)
        if voice_enables[2] {
            result += (self.voice3.output_volume() as f32 / 100.0) * final_volume;
        }
        // Voice 4 (Noise)
        if voice_enables[3] {}

        self.output_buffer.push(result);
    }

    fn tick_length(&mut self) {
        self.voice1.tick_length();
        self.voice2.tick_length();
        self.voice3.tick_length();
    }

    fn tick_envelop(&mut self) {
        self.voice1.tick_envelope();
        self.voice2.tick_envelope();
    }

    fn tick_sweep(&mut self) {
        self.voice1.tick_sweep();
    }
}

fn set_bit(output: &mut u8, bit: u8, set: bool) {
    *output = (*output & (!bit)) | if set { 1 } else { 0 } << bit;
}

fn test_bit(value: u8, bit: u8) -> bool {
    let mask = 1 << bit;
    (value & mask) == mask
}
