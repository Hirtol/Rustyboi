use log::LevelFilter;
use log::*;
use rustyboi_core::emulator::{Emulator, CYCLES_PER_FRAME};

use rustyboi_core::{EmulatorOptionsBuilder, InputKey};
use sdl2::keyboard::Keycode;

use simplelog::{CombinedLogger, Config, ConfigBuilder, TermLogger, TerminalMode, WriteLogger};

use std::fs::read;

use std::time::{Duration, Instant};

use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::event::{Event, WindowEvent};

use rustyboi_core::emulator::EmulatorMode::CGB;
use rustyboi_core::hardware::ppu::palette::{DisplayColour, RGB};

use crate::state::{AppEmulatorState, AppState};
use crossbeam::channel::*;
use rustyboi_core::hardware::ppu::FRAMEBUFFER_SIZE;

use crate::rendering::imgui::ImguiBoi;
use crate::rendering::Renderer;

use crate::communication::{EmulatorNotification, EmulatorResponse, DebugMessage};
use rustyboi::storage::{FileStorage, Storage};
use std::sync::Arc;

use crate::gameboy::GameboyRunner;
use crate::benchmarking::Benchmarking;
use crate::options::AppOptions;
use gumdrop::Options;
use crate::rendering::immediate::ImmediateGui;
use crate::communication::EmulatorNotification::Debug;
use rustyboi_core::hardware::apu::SAMPLE_CYCLES;

mod communication;
mod gameboy;
mod rendering;
mod state;
mod benchmarking;
mod options;

const KIRBY_DISPLAY_COLOURS: DisplayColour = DisplayColour {
    black: RGB(44, 44, 150),
    dark_grey: RGB(119, 51, 231),
    light_grey: RGB(231, 134, 134),
    white: RGB(247, 190, 247),
};

const GBC_UNR_DISPLAY_COLOURS: DisplayColour = DisplayColour {
    white: RGB(255, 255, 255),
    light_grey: RGB(123, 255, 49),
    dark_grey: RGB(0, 99, 197),
    black: RGB(0, 0, 0),
};

const DEFAULT_DISPLAY_COLOURS: DisplayColour = DisplayColour {
    white: RGB(175, 203, 70),
    light_grey: RGB(121, 170, 109),
    dark_grey: RGB(34, 111, 95),
    black: RGB(8, 41, 85),
};

const CONFIG_FILENAME: &str = "config.json";
const FPS: u64 = 60;
const FRAME_DELAY: Duration = Duration::from_nanos(1_000_000_000u64 / FPS);
const FAST_FORWARD_MULTIPLIER: u32 = 40;
// 0.25*44100*4 = 250 ms of delay at worst, average ~100 ms
const MAX_AUDIO_SAMPLES: u32 = 44100;
const MIN_AUDIO_SAMPLES: u32 = 8820;
const AUDIO_FREQUENCY: i32 = 44100;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Trace, ConfigBuilder::new().set_location_level(LevelFilter::Off).set_time_level(LevelFilter::Off).set_target_level(LevelFilter::Off).build(), std::io::BufWriter::new(File::create("rustyboi.log").unwrap())),
    ]).unwrap();
    // We want the first click on a gui element to actually register
    sdl2::hint::set("SDL_MOUSE_FOCUS_CLICKTHROUGH", "1");
    sdl2::hint::set_video_minimize_on_focus_loss(false);

    let options: AppOptions = AppOptions::parse_args_default_or_exit();

    let file_storage = Arc::new(FileStorage::new().unwrap());
    let mut app_state: AppState = file_storage.get_value(CONFIG_FILENAME).unwrap_or_default();

    let sdl_context = sdl2::init().expect("Failed to initialise SDL context!");
    let audio_subsystem = sdl_context.audio().expect("SDL context failed to initialise audio!");
    let video_subsystem = sdl_context.video().expect("SDL context failed to initialise video!");

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

    crate::benchmarking::run_benchmark(&options);

    let mut renderer: Renderer<ImguiBoi> = Renderer::new(video_subsystem, file_storage.clone()).unwrap();
    renderer.setup_immediate_gui("Rustyboi Debug");
    renderer.main_window.window_mut().raise();

    let _bootrom_file_dmg = read("roms/DMG_ROM.bin").unwrap();
    let bootrom_file_cgb = read("roms\\cgb_bios.bin").unwrap();

    let cartridge = "roms/Zelda.gb";
    let _yellow = "roms/Pokemon - Yellow Version.gbc";
    let _cpu_test = "test roms/auto-run/window_y_trigger.gb";
    let _cpu_test2 = "test roms/auto-run/hdma_timing-C.gbc";

    //Things to do:
    // 1: APU improvements to use a proper sampler so that we can re-architect the way we do ticking
    // by doing more lazy evaluation (thus being able to move everything to the scheduler for speed)
    // 2: Render GB games when running in GBC with GBC renderer, since bootrom sets custom palettes!
    let mut timer = sdl_context.timer().unwrap();
    let emu_opts = EmulatorOptionsBuilder::new()
        .boot_rom(Some(bootrom_file_cgb))
        .with_mode(CGB)
        .with_display_colours(KIRBY_DISPLAY_COLOURS)
        .build();
    let mut gameboy_runner = GameboyRunner::new(cartridge, emu_opts);
    let mut audio_player = AudioPlayer::new(audio_queue, Duration::from_millis(100));

    let mut loop_cycles = 0;

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut last_update_time: Instant = Instant::now();

    let mut emulation_state = AppEmulatorState::default();

    let mut most_recent_frame: [RGB; FRAMEBUFFER_SIZE] = [RGB::default(); FRAMEBUFFER_SIZE];
    audio_player.start();

    'mainloop: loop {
        audio_player.send_requests(&gameboy_runner);

        if let Some(requests) = renderer.render_immediate_gui(&event_pump) {
            if !emulation_state.awaiting_debug {
                requests.into_iter()
                    .map(DebugMessage::into)
                    .for_each(|r| {
                        gameboy_runner.request_sender.send(r);
                    });
                emulation_state.awaiting_debug = true;
            }
        }

        let ticks = timer.ticks() as i32;

        for event in event_pump.poll_iter() {
            if !handle_events(event, &mut gameboy_runner, &mut emulation_state, &mut renderer) {
                break 'mainloop;
            }
        }

        let mut frames_to_go = if emulation_state.fast_forward {
            app_state.fast_forward_rate
        } else {
            1
        };

        // I should really figure out proper audio syncing ._.
        if emulation_state.unbounded || emulation_state.fast_forward || !audio_player.has_too_many_samples() {
            for _ in 0..frames_to_go {
                if !emulation_state.emulator_paused {
                    most_recent_frame = gameboy_runner.frame_receiver.recv().unwrap();
                }
                renderer.render_main_window(&most_recent_frame);
            }
            loop_cycles += frames_to_go;
        }

        while let Ok(response) = gameboy_runner.response_receiver.try_recv() {
            match response {
                EmulatorResponse::Audio(buffer) => {
                    if audio_player.receive_audio(buffer) {
                        loop_cycles += 1;
                    }
                }
                EmulatorResponse::Debug(response) => {
                    if let Some(imgui) = renderer.immediate_gui.as_mut() {
                        imgui.fulfill_query(response);
                    }
                    emulation_state.awaiting_debug = false;
                }
            }
        }

        if last_update_time.elapsed().as_millis() >= 1000 {
            renderer.main_window.window_mut().set_title(
                format!(
                    "RustyBoi - {:.2} FPS",
                    (loop_cycles as f64 /  last_update_time.elapsed().as_secs_f64())
                )
                .as_str(),
            );
            last_update_time = Instant::now();
            loop_cycles = 0;
        }
        // Ideally we'd use Instant instead of SDL timer, but for some reason when using Instant
        // we sleep more than we should, leaving us at ~58 fps which causes audio stutters.
        let frame_time = timer.ticks() as i32 - ticks;

        if (!emulation_state.unbounded || emulation_state.emulator_paused) && FRAME_DELAY.as_millis() as i32 > frame_time {
            let sleep_time = (FRAME_DELAY.as_millis() as i32 - frame_time) as u64;
            std::thread::sleep(Duration::from_millis(sleep_time));
        }
    }

    file_storage.save_value(CONFIG_FILENAME, &app_state);
}

struct AudioPlayer {
    pub awaiting_audio: bool,
    sdl_audio: AudioQueue<f32>,
    channel_queue: Vec<f32>,
}

impl AudioPlayer {
    /// Creates a new audio player for an SDL `AudioQueue`.
    ///
    /// Will start the queue by playing `initial_buffer_length` (millisecond accuracy)
    /// silence as a buffer to avoid initial crackle.
    pub fn new(sdl_audio: AudioQueue<f32>, initial_buffer_length: Duration) -> Self {
        let silence_samples = initial_buffer_length.as_secs_f64() * AUDIO_FREQUENCY as f64;
        sdl_audio.queue(&vec![0.0; silence_samples as usize]);
        AudioPlayer{
            awaiting_audio: false,
            sdl_audio,
            channel_queue: Vec::with_capacity(5000),
        }
    }

    pub fn start(&self) {
        self.sdl_audio.resume();
    }

    pub fn pause(&self) {
        self.sdl_audio.pause()
    }

    #[inline]
    pub fn has_enough_samples(&self) -> bool {
        self.sdl_audio.size() >= MIN_AUDIO_SAMPLES
    }

    #[inline]
    pub fn has_too_many_samples(&self) -> bool {
        self.sdl_audio.size() >= MAX_AUDIO_SAMPLES
    }

    pub fn send_requests(&mut self, gameboy_runner: &GameboyRunner) {
        if !self.awaiting_audio && !self.has_too_many_samples() {
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

fn handle_events(
    event: Event,
    gameboy_runner: &mut GameboyRunner,
    app_state: &mut AppEmulatorState,
    renderer: &mut Renderer<ImguiBoi>,
) -> bool {
    if handle_debug_window_events(&event, renderer) {
        return true;
    }
    match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            window_id: 1,
            ..
        }
        | Event::Window {
            window_id: 1,
            win_event: WindowEvent::Close,
            ..
        } => {
            gameboy_runner.stop();
            app_state.exit = true;
            return false;
        }
        Event::Window {
            window_id: _,
            win_event: WindowEvent::Close,
            ..
        }
        | Event::KeyDown {keycode: Some(Keycode::Escape), ..} => {
            renderer.close_immediate_gui();
            renderer.main_window.window_mut().raise();
        }
        Event::DropFile { filename, .. } => {
            if filename.ends_with(".gb") || filename.ends_with(".gbc") {
                debug!("Opening file: {}", filename);

                app_state.reset();
                gameboy_runner.stop();

                let emu_opts = EmulatorOptionsBuilder::new()
                    .with_mode(CGB)
                    .with_display_colours(KIRBY_DISPLAY_COLOURS)
                    .build();
                *gameboy_runner = GameboyRunner::new(&filename, emu_opts);
            } else {
                warn!("Attempted opening of file: {} which is not a GameBoy rom!", filename);
            }
        }
        Event::KeyDown { keycode: Some(key), window_id: 1, .. } => {
            if let Some(input_key) = keycode_to_input(key) {
                gameboy_runner.handle_input(input_key, true);
            } else {
                match key {
                    Keycode::LShift => app_state.fast_forward = true,
                    Keycode::U => app_state.unbounded = !app_state.unbounded,
                    Keycode::P => app_state.emulator_paused = !app_state.emulator_paused,
                    Keycode::K => renderer.setup_immediate_gui("Rustyboi Debugging").unwrap(),
                    Keycode::F11 => renderer.toggle_main_window_fullscreen(),
                    // Keycode::O => println!("{:#?}", notifier.oam()),
                    // Keycode::L => {
                    //     let mut true_image_buffer = vec![0u8; 768*8*8*3];
                    //
                    //     for (i, colour) in notifier.vram_tiles().iter().enumerate() {
                    //         let offset = i * 3;
                    //         true_image_buffer[offset] = colour.0;
                    //         true_image_buffer[offset + 1] = colour.1;
                    //         true_image_buffer[offset + 2] = colour.2;
                    //     }
                    //     let temp_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
                    //         image::ImageBuffer::from_raw(128, 384, true_image_buffer).unwrap();
                    //     let temp_buffer = image::imageops::resize(&temp_buffer, 256, 768, FilterType::Nearest);
                    //     temp_buffer
                    //         .save(format!("vram_dump.png"))
                    //         .unwrap();
                    // }
                    _ => {}
                }
            }
        }
        Event::KeyUp { keycode: Some(key), window_id: 1, .. } => {
            if let Some(input_key) = keycode_to_input(key) {
                gameboy_runner.handle_input(input_key, false);
            } else {
                match key {
                    Keycode::LShift => app_state.fast_forward = false,
                    _ => {}
                }
            }
        }
        _ => {}
    }

    true
}

fn handle_debug_window_events(event: &Event, renderer: &mut Renderer<ImguiBoi>) -> bool {
    if let Some(gui) = &mut renderer.immediate_gui {
        let second_window_id = renderer.debug_window.as_ref().unwrap().id();

        if !event.get_window_id().is_some() || (event.get_window_id().unwrap() == second_window_id) {
            gui.handle_event(event);
            // Since the event was meant for the second window we'll always ignore it.
            // If the GUI was on the main window we should use input_handler.ignore_event().
            if gui.input_handler.ignore_event(event) {
                return true;
            }
        }
    }
    false
}

fn keycode_to_input(key: Keycode) -> Option<InputKey> {
    match key {
        Keycode::Up => Some(InputKey::UP),
        Keycode::Down => Some(InputKey::DOWN),
        Keycode::Left => Some(InputKey::LEFT),
        Keycode::Right => Some(InputKey::RIGHT),
        Keycode::A => Some(InputKey::A),
        Keycode::B => Some(InputKey::B),
        Keycode::S => Some(InputKey::SELECT),
        Keycode::T => Some(InputKey::START),
        _ => None,
    }
}
