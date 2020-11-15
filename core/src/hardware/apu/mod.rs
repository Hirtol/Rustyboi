//! This entire APU implementation currently passes the first 6/12 Blargg DMG_SOUND tests.
//! However, there are still several other issues such as the wave_channel not always behaving
//! properly. In addition, sound completely stops in the game Aladdin after the title screen for
//! some reason.

use crate::emulator::{EmulatorMode, DMG_CLOCK_SPEED};
use crate::hardware::apu::noise_channel::NoiseChannel;
use crate::hardware::apu::square_channel::SquareWaveChannel;
use crate::hardware::apu::wave_channel::WaveformChannel;
use crate::hardware::mmu::INVALID_READ;
use crate::scheduler::{EventType, Scheduler};

mod channel_features;
mod noise_channel;
mod square_channel;
mod wave_channel;

// Currently chose for 44100/60 = 739 samples per frame to make it 'kinda' sync up.
// In all likelihood this will cause issues due to scheduling delays so this should go up probably.
pub const SAMPLE_SIZE_BUFFER: usize = 739;
pub const FRAME_SEQUENCE_CYCLES: u64 = 8192;
pub const SAMPLE_CYCLES: u64 = 95;

pub const APU_MEM_START: u16 = 0xFF10;
pub const APU_MEM_END: u16 = 0xFF2F;
pub const WAVE_SAMPLE_START: u16 = 0xFF30;
pub const WAVE_SAMPLE_END: u16 = 0xFF3F;

#[derive(Debug)]
pub struct APU {
    voice1: SquareWaveChannel,
    voice2: SquareWaveChannel,
    voice3: WaveformChannel,
    voice4: NoiseChannel,
    audio_output: AudioOutput,
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
    frame_sequencer_step: u8,
    last_access_point: u64,
}

impl APU {
    pub fn new() -> Self {
        APU {
            voice1: SquareWaveChannel::new(),
            voice2: SquareWaveChannel::new(),
            voice3: WaveformChannel::new(),
            voice4: NoiseChannel::new(),
            audio_output: AudioOutput::default(),
            vin_l_enable: false,
            vin_r_enable: false,
            left_volume: 7,
            right_volume: 7,
            left_channel_enable: [true; 4],
            right_channel_enable: [true, true, false, false],
            // Start the APU with 2 frames of audio buffered
            output_buffer: Vec::with_capacity(SAMPLE_SIZE_BUFFER * 2),
            global_sound_enable: true,
            frame_sequencer_step: 0,
            last_access_point: 0
        }
    }

    /// Tick all channels, not including the frame sequencer and sampler.
    pub fn synchronise(&mut self, scheduler: &mut Scheduler, speed_multiplier: u64) {
        if !self.global_sound_enable {
            return;
        }

        let delta = (scheduler.current_time - self.last_access_point) >> speed_multiplier;
        let (mut samples, remainder) = (delta / self.audio_output.cycles_per_sample, delta % self.audio_output.cycles_per_sample);

        self.last_access_point = scheduler.current_time;

        while samples > 0 {
            self.voice1.tick_timer(self.audio_output.cycles_per_sample - self.audio_output.remainder_cycles_sample);
            self.voice2.tick_timer(self.audio_output.cycles_per_sample - self.audio_output.remainder_cycles_sample);
            self.voice3.tick_timer(self.audio_output.cycles_per_sample - self.audio_output.remainder_cycles_sample);
            self.voice4.tick_timer(self.audio_output.cycles_per_sample - self.audio_output.remainder_cycles_sample);
            self.audio_output.remainder_cycles_sample = 0;
            self.tick_sampling_handler();
            samples -= 1;
        }

        self.audio_output.remainder_cycles_sample = remainder;
        self.voice1.tick_timer(remainder);
        self.voice2.tick_timer(remainder);
        self.voice3.tick_timer(remainder);
        self.voice4.tick_timer(remainder);

        #[cfg(feature = "apu-logging")]
        log::debug!("Voice 3, remaining timer: {} - cycles: {} - scheduler time: {}", self.voice3.timer, self.voice3.cycles_done, scheduler.current_time);
    }

    /// Ticked by the `Scheduler` every `8192` cycles.
    ///
    /// See `MMU` for function call.
    pub fn tick_frame_sequencer(&mut self, scheduler: &mut Scheduler, speed_multiplier: u64) {
        self.synchronise(scheduler, speed_multiplier);
        // The frame sequencer component clocks at 512Hz apparently.
        // 4194304/512 = 8192 cycles
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

    /// Ticked by the `Scheduler` every `95` cycles.
    /// This is a close enough value such that we get one sample every ~1/44100 seconds
    ///
    /// See `MMU` for function call.
    pub fn tick_sampling_handler(&mut self) {
        //self.synchronise(scheduler, speed_multiplier);
        // This block is here such that we get ~44100 samples per second, otherwise we'd generate
        // far more than we could consume.
        // TODO: Add actual downsampling instead of the selective audio pick.
        // Refer to: https://www.reddit.com/r/EmuDev/comments/g5czyf/sound_emulation/
        // Alternatively, we could go to 93207 sampling rate, which would give the sampling
        // handler a value of *almost* exactly 45.

        // If we ever want to implement a low pass filter we would probably have to generate
        // samples at native rate (so every 4/8 clocks) in each individual channel. Could consider
        // trying SIMD then?

        // These values are purely personal preference, may even want to defer this to the emulator
        // consumer.
        let left_final_volume = self.left_volume as f32 / 6.0;
        let right_final_volume = self.right_volume as f32 / 6.0;

        // Left Audio
        let left_sample = self.generate_audio(self.left_channel_enable, left_final_volume);
        // Right Audio
        let right_sample = self.generate_audio(self.right_channel_enable, right_final_volume);

        let result_samples = self.audio_output.apply_highpass_filter(left_sample, right_sample);

        self.output_buffer.push(result_samples.0);
        self.output_buffer.push(result_samples.1);
    }

    pub fn get_audio_buffer(&self) -> &[f32] {
        &self.output_buffer
    }

    pub fn clear_audio_buffer(&mut self) {
        self.output_buffer.clear();
    }

    pub fn read_register(&mut self, address: u16, scheduler: &mut Scheduler, speed_multiplier: u64) -> u8 {
        self.synchronise(scheduler, speed_multiplier);
        let address = address & 0xFF;
        //TODO: No read if disabled?
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
            }
            0x27..=0x2F => INVALID_READ, // Unused registers, always read 0xFF
            _ => unreachable!("Out of bound APU register read: {}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8, scheduler: &mut Scheduler, mode: EmulatorMode, speed_multiplier: u64) {
        self.synchronise(scheduler, speed_multiplier);
        //log::info!("APU Write on address: 0x{:02X} with value: 0x{:02X}", address, value);
        let address = address & 0xFF;

        // It's not possible to access any registers beside 0x26 while the sound is disabled.
        // *Caveat*: In DMG mode you CAN write to the Length registers while disabled (f.e 0x20).
        // However in CGB mode this is not possible, and should thus not be allowed.
        if !self.global_sound_enable
            && address != 0x26
            && (mode.is_cgb() || (mode.is_dmg() && ![0x20, 0x1B].contains(&address)))
        {
            log::warn!("Tried to write to APU while inaccessible at address: 0x{:02X}", address);
            return;
        }

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
                let previous_enable = self.global_sound_enable;
                self.global_sound_enable = test_bit(value, 7);
                if !self.global_sound_enable {
                    self.reset(scheduler, mode);
                } else if !previous_enable {
                    // Re-add the frame sequence event.
                    scheduler.push_relative(EventType::APUFrameSequencer, FRAME_SEQUENCE_CYCLES);
                    scheduler.push_relative(EventType::APUSample, SAMPLE_CYCLES);
                }
            }
            0x27..=0x2F => {} // Writes to unused registers are silently ignored.
            _ => unreachable!(
                "Attempt to write to an unknown audio register: 0xFF{:02X} with val: {}",
                address, value
            ),
        }
    }

    pub fn read_wave_sample(&mut self, address: u16, scheduler: &mut Scheduler, speed_multiplier: u64) -> u8 {
        self.synchronise(scheduler, speed_multiplier);
        let address = address & 0xFF;
        self.voice3.read_register(address)
    }

    pub fn write_wave_sample(&mut self, address: u16, value: u8, scheduler: &mut Scheduler, speed_multiplier: u64) {
        self.synchronise(scheduler, speed_multiplier);
        //log::info!("APU Wave_Write on address: 0x{:02X} with value: 0x{:02X}", address, value);
        let address = address & 0xFF;
        self.voice3.write_register(address, value, self.frame_sequencer_step)
    }

    fn generate_audio(&mut self, voice_enables: [bool; 4], final_volume: f32) -> f32{
        let mut result = 0f32;
        // Voice 1 (Square wave)
        if voice_enables[0] {
            result += (self.voice1.output_volume() as f32);
        }
        // Voice 2 (Square wave)
        if voice_enables[1] {
            result += (self.voice2.output_volume() as f32);
        }
        // Voice 3 (Wave)
        if voice_enables[2] {
            result += (self.voice3.output_volume() as f32);
        }
        // Voice 4 (Noise)
        if voice_enables[3] {
            result += (self.voice4.output_volume() as f32);
        }
        //TODO: Move / 100.0 after high pass.
        (result / 100.0) * final_volume
    }

    fn tick_length(&mut self) {
        if self.global_sound_enable {
            self.voice1.tick_length();
            self.voice2.tick_length();
            self.voice3.tick_length();
            self.voice4.tick_length();
        }
    }

    fn tick_envelop(&mut self) {
        self.voice1.tick_envelope();
        self.voice2.tick_envelope();
        self.voice4.tick_envelope();
    }

    fn tick_sweep(&mut self) {
        self.voice1.tick_sweep();
    }

    fn reset(&mut self, scheduler: &mut Scheduler, mode: EmulatorMode) {
        self.voice1.reset(mode);
        self.voice2.reset(mode);
        self.voice3.reset();
        self.voice4.reset(mode);
        self.vin_l_enable = false;
        self.vin_r_enable = false;
        self.right_volume = 0;
        self.left_volume = 0;
        self.left_channel_enable = [false; 4];
        self.right_channel_enable = [false; 4];
        self.frame_sequencer_step = 0;
        scheduler.remove_event_type(EventType::APUFrameSequencer);
        scheduler.remove_event_type(EventType::APUSample);
    }
}

#[derive(Debug)]
pub struct AudioOutput {
    remainder_cycles_sample: u64,
    cycles_per_sample: u64,
    highpass_rate: f32,
    highpass_diff: (f32, f32)
}

impl Default for AudioOutput {
    fn default() -> Self {
        AudioOutput {
            remainder_cycles_sample: 0,
            cycles_per_sample: SAMPLE_CYCLES,
            highpass_rate: get_highpass_rate(SAMPLE_CYCLES),
            highpass_diff: (0.0, 0.0)
        }
    }
}

impl AudioOutput {
    #[inline]
    pub fn apply_highpass_filter(&mut self, left_in: f32, right_in: f32) -> (f32, f32) {
        // Credits to SameBoy since I looked at their implementation for this.
        let (high_left, high_right) = self.highpass_diff;
        let (filt_left, filt_right) = (left_in - high_left , right_in - high_right);
        self.highpass_diff = (left_in - (filt_left * self.highpass_rate), right_in - (filt_right * self.highpass_rate));
        (filt_left, filt_right)
    }

    pub fn set_sample_rate(&mut self, sample_rate_in_hz: u64) {
        self.cycles_per_sample = DMG_CLOCK_SPEED / sample_rate_in_hz;
        self.highpass_rate = get_highpass_rate(self.cycles_per_sample);
    }
}

fn no_length_tick_next_step(next_frame_sequence_val: u8) -> bool {
    // Due to the fact that we increment frame_sequencer immediately we have to check for current_step + 1
    [1, 3, 5, 7].contains(&next_frame_sequence_val)
}

fn get_highpass_rate(cycles_per_sample: u64) -> f32 {
    0.999958f32.powf(cycles_per_sample as f32)
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
