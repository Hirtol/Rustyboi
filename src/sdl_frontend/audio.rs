use std::time::Duration;
use crate::{AUDIO_FREQUENCY, MIN_AUDIO_SAMPLES, MAX_AUDIO_SAMPLES};
use crate::gameboy::GameboyRunner;
use crate::communication::EmulatorNotification;
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::AudioSubsystem;

pub struct AudioPlayer {
    awaiting_audio: bool,
    paused: bool,
    sdl_audio: AudioQueue<f32>,
    channel_queue: Vec<f32>,
}

impl AudioPlayer {
    /// Creates a new audio player for an SDL `AudioQueue`.
    ///
    /// Will start the queue by playing `initial_buffer_length` (millisecond accuracy)
    /// silence as a buffer to avoid initial crackle.
    pub fn new(audio_subsystem: &AudioSubsystem, initial_buffer_length: Duration) -> Self {
        let audio_queue: AudioQueue<f32> = audio_subsystem
            .open_queue(
                None,
                &AudioSpecDesired {
                    freq: Some(AUDIO_FREQUENCY),
                    channels: Some(2),
                    samples: None,
                },
            )
            .unwrap();
        let silence_samples = initial_buffer_length.as_secs_f64() * AUDIO_FREQUENCY as f64;
        audio_queue.queue(&vec![0.0; silence_samples as usize]);
        AudioPlayer{
            awaiting_audio: false,
            paused: true,
            sdl_audio: audio_queue,
            channel_queue: Vec::with_capacity(5000),
        }
    }

    pub fn start(&mut self) {
        self.paused = false;
        self.sdl_audio.resume();
    }

    pub fn pause(&mut self) {
        self.paused = true;
        self.sdl_audio.pause()
    }

    pub fn reset(&mut self) {
        self.awaiting_audio = false;
        self.channel_queue = Vec::with_capacity(5000);
    }

    #[inline]
    pub fn has_enough_samples(&self) -> bool {
        self.sdl_audio.size() >= MIN_AUDIO_SAMPLES
    }

    #[inline]
    pub fn has_too_many_samples(&self) -> bool {
        self.sdl_audio.size() >= MAX_AUDIO_SAMPLES
    }

    /// Send audio requests to the emulator thread as appropriate,
    /// expects to be called *at least* once every 1/60th of a second.
    pub fn send_requests(&mut self, gameboy_runner: &GameboyRunner) {
        if !self.awaiting_audio && !self.has_too_many_samples() && !self.paused {
            let buffer_to_send = std::mem::replace(&mut self.channel_queue, Vec::new());
            gameboy_runner
                .request_sender
                .send(EmulatorNotification::AudioRequest(buffer_to_send));
            if !self.has_enough_samples() {
                gameboy_runner
                    .request_sender
                    .send(EmulatorNotification::ExtraAudioRequest);
            }

            self.awaiting_audio = true;
        }
    }

    /// Receive and play a audio buffer from the emulator
    ///
    /// # Returns
    /// Will return `true` if we asked for an additional catch-up frame to be run
    /// so that we won't starve the audio buffer.
    pub fn receive_audio(&mut self, mut received_buffer: Vec<f32>) -> bool {
        self.sdl_audio.queue(&received_buffer);
        received_buffer.clear();
        if received_buffer.capacity() > self.channel_queue.capacity() {
            self.channel_queue = received_buffer;
        }
        if self.awaiting_audio {
            self.awaiting_audio = false;
            false
        } else {
            // We executed an extra frame to catch up with the audio.
            true
        }
    }
}
