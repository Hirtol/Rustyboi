use crate::communication::{EmulatorNotification, EmulatorResponse, DebugMessage};
use core::option::Option::Some;
use crossbeam::channel::*;
use rustyboi::actions::{create_emulator, save_rom};
use rustyboi_core::emulator::Emulator;
use rustyboi_core::hardware::ppu::palette::RGB;
use rustyboi_core::hardware::ppu::FRAMEBUFFER_SIZE;
use rustyboi_core::{EmulatorOptions, InputKey};
use std::path::Path;
use std::thread::JoinHandle;

pub struct GameboyRunner {
    current_thread: Option<JoinHandle<()>>,
    pub frame_receiver: Receiver<[RGB; FRAMEBUFFER_SIZE]>,
    pub request_sender: Sender<EmulatorNotification>,
    pub response_receiver: Receiver<EmulatorResponse>,
}

impl GameboyRunner {
    pub fn new(rom_path: impl AsRef<Path>, options: EmulatorOptions) -> GameboyRunner {
        let (frame_sender, frame_receiver) = bounded(1);
        let (request_sender, request_receiver) = unbounded::<EmulatorNotification>();
        let (response_sender, response_receiver) = unbounded::<EmulatorResponse>();
        let mut emulator = create_emulator(rom_path, options);
        let emulator_thread =
            std::thread::spawn(move || {
                run_emulator(&mut emulator, frame_sender, response_sender, request_receiver);
                save_rom(&emulator);
            });
        GameboyRunner {
            current_thread: Some(emulator_thread),
            frame_receiver,
            request_sender,
            response_receiver,
        }
    }

    pub fn is_running(&self) -> bool {
        self.current_thread.is_some()
    }

    pub fn handle_input(&self, key: InputKey, pressed: bool) {
        //TODO: Error handling
        if pressed {
            self.request_sender.send(EmulatorNotification::KeyDown(key));
        } else {
            self.request_sender.send(EmulatorNotification::KeyUp(key));
        }
    }

    /// Stops the current emulator thread and blocks until it has completed.
    ///
    /// Commands the emulator thread to save the current saves to disk as well.
    pub fn stop(&mut self) {
        if let Some(thread) = self.current_thread.take() {
            self.request_sender.send(EmulatorNotification::ExitRequest);
            // Since the emulation thread may be blocking trying to send a frame.
            self.frame_receiver.try_recv();
            thread.join();
        }
    }
}

fn run_emulator(
    emulator: &mut Emulator,
    frame_sender: Sender<[RGB; FRAMEBUFFER_SIZE]>,
    response_sender: Sender<EmulatorResponse>,
    notification_receiver: Receiver<EmulatorNotification>,
) {
    'emu_loop: loop {
        while !emulator.emulate_cycle() {}

        if let Err(e) = frame_sender.send(emulator.frame_buffer().clone()) {
            log::error!("Failed to transfer framebuffer due to: {:?}", e);
            break 'emu_loop;
        }

        while let Ok(notification) = notification_receiver.try_recv() {
            match notification {
                EmulatorNotification::KeyDown(key) => emulator.handle_input(key, true),
                EmulatorNotification::KeyUp(key) => emulator.handle_input(key, false),
                EmulatorNotification::AudioRequest(mut audio_buffer) => {
                    audio_buffer.extend(emulator.audio_buffer().iter());
                    if let Err(e) = response_sender.send(EmulatorResponse::Audio(audio_buffer)) {
                        log::error!("Failed to transfer audio buffer due to: {:?}", e);
                        break 'emu_loop;
                    }
                }
                EmulatorNotification::Debug(request) => {
                    if !handle_debug_request(request, emulator, &response_sender) {
                        break 'emu_loop;
                    }
                },
                EmulatorNotification::ExitRequest => {
                    break 'emu_loop;
                }
            }
        }
        // Since we know that in the common runtime the emulator thread will run in lockstep
        // with the rendering thread we can safely clear the audio buffer here.
        // When running in fast forward we'll get a cool audio speedup effect.
        emulator.clear_audio_buffer();
    }
}

fn handle_debug_request(request: DebugMessage, emulator: &mut Emulator,
                        response_sender: &Sender<EmulatorResponse>) -> bool {
    let response;
    match request {
        DebugMessage::Palette(_) => {
            response = response_sender.send(DebugMessage::Palette(emulator.get_palette_info().into()).into());
        }
    }

    if let Err(e) = response {
        log::error!("Failed sending of palette info to debug request due to: {}", e);
        false
    } else {
        true
    }
}
