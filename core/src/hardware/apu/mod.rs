use crate::hardware::apu::channels::Voice1;

mod channels;
mod memory_binds;
mod channel_features;

// Currently chose for 44100/60 = 739 samples per frame to make it 'kinda' sync up.
// In all likelihood this will cause issues due to scheduling delays so this should go up probably.
pub const SAMPLE_SIZE_BUFFER: usize = 739;

pub const APU_MEM_START: u16 = 0xFF10;
pub const APU_MEM_END: u16 = 0xFF26;

pub struct APU {
    voice1: Voice1,
    /// The volume bits specify the "Master Volume" for Left/Right sound output.
    /// SO2 goes to the left headphone, and SO1 goes to the right.
    ///
    /// The Vin signal is unused by licensed games (could've been used for 5th voice)
    ///
    ///  ```Bit 7   - Output Vin to SO2 terminal (1=Enable)
    ///  Bit 6-4 - SO2 output level (volume)  (0-7)
    ///  Bit 3   - Output Vin to SO1 terminal (1=Enable)
    ///  Bit 2-0 - SO1 output level (volume)  (0-7)```
    //nr50: u8,
    /// Each channel can be panned hard left, center, or hard right.
    ///
    ///  ```Bit 7 - Output sound 4 to SO2 terminal
    ///  Bit 6 - Output sound 3 to SO2 terminal
    ///  Bit 5 - Output sound 2 to SO2 terminal
    ///  Bit 4 - Output sound 1 to SO2 terminal
    ///  Bit 3 - Output sound 4 to SO1 terminal
    ///  Bit 2 - Output sound 3 to SO1 terminal
    ///  Bit 1 - Output sound 2 to SO1 terminal
    ///  Bit 0 - Output sound 1 to SO1 terminal```
    //nr51: u8,
    /// Disabling the sound controller by clearing Bit 7 destroys the contents of all sound registers.
    /// Also, it is not possible to access any sound registers (execpt FF26) while the sound controller is disabled.
    /// Bits 0-3 of this register are read only status bits, writing to these bits does NOT enable/disable sound.
    /// The flags get set when sound output is restarted by setting the Initial flag (Bit 7 in NR14-NR44),
    /// the flag remains set until the sound length has expired (if enabled).
    /// A volume envelopes which has decreased to zero volume will NOT cause the sound flag to go off.
    ///
    ///  ```Bit 7 - All sound on/off  (0: stop all sound circuits) (Read/Write)
    ///  Bit 3 - Sound 4 ON flag (Read Only)
    ///  Bit 2 - Sound 3 ON flag (Read Only)
    ///  Bit 1 - Sound 2 ON flag (Read Only)
    ///  Bit 0 - Sound 1 ON flag (Read Only)```
    // nr52: u8
    /// All sound on/off  (0: stop all sound circuits) (Read/Write)
    all_sound_enable: bool,

    left_volume: u8,
    right_volume: u8,
    // 0-3 will represent voice 1-4 enable respectively.
    left_channel_enable: [bool; 4],
    right_channel_enable: [bool; 4],

    output_buffer: Vec<f32>,

    sampling_handler: u8,
    frame_sequencer: u16,
    frame_sequencer_step: u8,
}

impl APU {
    pub fn new() -> Self {
        APU {
            voice1: Default::default(),
            left_volume: 7,
            right_volume: 7,
            left_channel_enable: [true; 4],
            right_channel_enable: [true, true, false, false],
            // Start the APU with 2 frames of audio buffered
            output_buffer: vec![0f32; SAMPLE_SIZE_BUFFER * 2],
            frame_sequencer: 0,
            sampling_handler: 0,
            all_sound_enable: true,
            frame_sequencer_step: 0,
        }
    }

    pub fn tick(&mut self, mut delta_cycles: u64) {
        if !self.all_sound_enable {
            return;
        }
        //TODO: Adjust these volumes to our liking.
        let left_final_volume = self.left_volume as f32 / 6.0;
        let right_final_volume = self.right_volume as f32 / 6.0;

        while delta_cycles > 0 {
            self.voice1.tick_timer();

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
        if !self.all_sound_enable && address != 0x26 {
            log::warn!("Tried to read APU while inaccessible");
            return 0xFF;
        }

        match address {
            0x10..=0x14 => self.voice1.read_register(address),
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
                let mut output = 0u8;
                set_bit(&mut output, 7, self.all_sound_enable);
                //TODO: These three voices enable flags.
                set_bit(&mut output, 3, true);
                set_bit(&mut output, 2, true);
                set_bit(&mut output, 1, true);

                set_bit(&mut output, 0, self.voice1.enabled());
                output
            }
            //TODO: Once all voices are implemented bring back panic.
            _ => 0xFF//panic!("Attempt to read an unknown audio register: 0xFF{:02X}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        let address = address & 0xFF;

        // It's not possible to access any registers beside 0x26 while the sound is disabled.
        if !self.all_sound_enable && address != 0x26 {
            log::warn!("Tried to write APU while inaccessible");
            return;
        }

        match address {
            0x10..=0x14 => self.voice1.write_register(address, value),
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
                self.all_sound_enable = (value & 0b1000_0000) == 0b1000_0000;
                self.voice1 = Voice1::default();
                //TODO: Reset all voices.
            }
            //TODO: Once all voices are implemented bring back panic.
            _ => {}//panic!("Attempt to write to an unknown audio register: 0xFF{:02X} with val: {}", address, value),
        }
    }

    fn generate_audio(&mut self, voice_enables: [bool; 4], final_volume: f32) {
        let mut result = 0f32;
        // Voice 1 (Square wave)
        if voice_enables[0] {
            result += (self.voice1.output_volume() as f32 / 100.0) * final_volume;
        }
        // Voice 2 (Square wave)
        if voice_enables[1] {}
        // Voice 3 (Wave)
        if voice_enables[2] {}
        // Voice 4 (Noise)
        if voice_enables[3] {}

        self.output_buffer.push(result);
    }

    fn tick_length(&mut self) {
        self.voice1.tick_length();
    }

    fn tick_envelop(&mut self) {
        self.voice1.envelope.tick();
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

