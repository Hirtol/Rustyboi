use log::LevelFilter;
use log::*;
use rustyboi_core::emulator::{Emulator, CYCLES_PER_FRAME};

use rustyboi_core::{InputKey, EmulatorOptionsBuilder, EmulatorOptions};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum::RGB24;
use sdl2::render::{Canvas, Texture, WindowCanvas};
use sdl2::video::{Window, GLProfile};
use sdl2::Sdl;
use simplelog::{CombinedLogger, Config, ConfigBuilder, TermLogger, TerminalMode, WriteLogger};

use std::fs::{read, File};

use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, Instant};

use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::event::{Event, WindowEvent};
use std::io::Write;
use std::ops::Div;
use rustyboi_core::emulator::EmulatorMode::{CGB, DMG};
use rustyboi_core::hardware::ppu::palette::{DisplayColour, RGB};
use image::ImageBuffer;
use image::imageops::FilterType;
use rustyboi::actions::{save_rom, create_emulator};
use crate::sdl::{setup_sdl, fill_texture_and_copy};
use crossbeam::channel::*;
use rustyboi_core::hardware::ppu::FRAMEBUFFER_SIZE;
use crate::state::AppEmulatorState;
use imgui::Context;
use crate::rendering::Renderer;
use crate::rendering::imgui::ImguiBoi;
use rustyboi::actions;
use rustyboi::storage::FileStorage;
use std::sync::Arc;
use crate::communication::{EmulatorResponse, EmulatorNotification};
use std::path::Path;


mod sdl;
mod state;
mod rendering;
mod communication;

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

const FPS: u64 = 60;
const FRAME_DELAY: Duration = Duration::from_nanos(1_000_000_000u64 / FPS);
const FAST_FORWARD_MULTIPLIER: u32 = 40;
const MAX_AUDIO_SAMPLES: u32 = 70_000;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Trace, ConfigBuilder::new().set_location_level(LevelFilter::Off).set_time_level(LevelFilter::Off).set_target_level(LevelFilter::Off).build(), std::io::BufWriter::new(File::create("rustyboi.log").unwrap())),
    ])
    .unwrap();
    let file_storage = Arc::new(FileStorage::new().unwrap());
    let sdl_context = sdl2::init().expect("Failed to initialise SDL context!");
    let audio_subsystem = sdl_context.audio().expect("SDL context failed to initialise audio!");
    let video_subsystem = sdl_context.video().expect("SDL context failed to initialise video!");

    let audio_queue: AudioQueue<f32> = audio_subsystem
        .open_queue(
            None,
            &AudioSpecDesired {
                freq: Some(44100),
                channels: Some(2),
                samples: None,
            },
        )
        .unwrap();

    let mut renderer: Renderer<ImguiBoi> = Renderer::new(video_subsystem, file_storage.clone()).unwrap();
    renderer.setup_immediate_gui("Rustyboi ImGui");

    let bootrom_file_dmg = read("roms/DMG_ROM.bin").unwrap();
    let bootrom_file_cgb = read("roms\\cgb_bios.bin").unwrap();

    let cartridge = "roms/Zelda.gb";
    let yellow = "roms/Pokemon - Yellow Version.gbc";
    let _cpu_test = "test roms/auto-run/window_y_trigger.gb";
    let _cpu_test2 = "test roms/auto-run/hdma_timing-C.gbc";

    //let mut emulator = Emulator::new(Option::Some(vec_to_bootrom(&bootrom_file)), &cartridge);

    // let (frame_sender, frame_receiver) = bounded(1);
    // std::thread::spawn(move || test_fast( &read(cartridge).unwrap(), frame_sender));
    // render_fast(&mut renderer, frame_receiver);
    //
    // return;

    //Things to do:
    // 1: APU improvements to use a proper sampler so that we can re-architect the way we do ticking
    // by doing more lazy evaluation (thus being able to move everything to the scheduler for speed)

    let mut timer = sdl_context.timer().unwrap();
    let emu_opts = EmulatorOptionsBuilder::new()
        //.boot_rom(Some(bootrom_file_cgb))
        .with_mode(CGB)
        .with_display_colours(KIRBY_DISPLAY_COLOURS)
        .build();

    let mut gameboy_runner = GameboyRunner::new(cartridge, emu_opts);

    let mut loop_cycles = 0;

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut last_update_time: Instant = Instant::now();

    let mut app_state = AppEmulatorState::default();
    let mut audio_buffer = Vec::with_capacity(5000);
    audio_queue.resume();

    'mainloop: loop {
        let audio_queue_size = audio_queue.size();
        if !app_state.awaiting_audio && audio_queue_size < MAX_AUDIO_SAMPLES {
            gameboy_runner.request_sender.send(EmulatorNotification::AudioRequest(audio_buffer));
            // Needed to satisfy the borrow checker
            audio_buffer = Vec::new();
            app_state.awaiting_audio = true;
        }

        let ticks = timer.ticks() as i32;

        for event in event_pump.poll_iter() {
            if let Some(gui) = &mut renderer.immediate_gui {
                let second_window_id = renderer.debug_window.as_ref().unwrap().id();
                if !event.get_window_id().is_some() || (event.get_window_id().unwrap() == second_window_id) {
                    gui.input_handler.handle_event(&mut gui.imgui_context, &event);
                    if gui.input_handler.ignore_event(&event) {
                        continue;
                    }
                }
            }

            if !handle_events(event, &mut gameboy_runner, &mut app_state, &mut renderer) {
                break 'mainloop;
            }
        }

        let frames_to_go = if app_state.fast_forward { FAST_FORWARD_MULTIPLIER } else { 1 };

        // 50k seems to be a decent spot for audio syncing.
        // I should really figure out proper audio syncing ._.
        if app_state.unbounded || audio_queue_size < MAX_AUDIO_SAMPLES {
            for _ in 0..frames_to_go {
                if !app_state.emulator_paused {
                    let framebuffer = gameboy_runner.frame_receiver.recv().unwrap();
                    renderer.render_main_window(&framebuffer);
                }
            }
        }

        renderer.render_immediate_gui(&event_pump);
        loop_cycles += frames_to_go;

        while let Ok(response) = gameboy_runner.response_receiver.try_recv() {
            match response {
                EmulatorResponse::AUDIO(mut buffer) => {
                    audio_queue.queue(&buffer);
                    buffer.clear();
                    audio_buffer = buffer;
                    app_state.awaiting_audio = false;
                },
            }
        }

        if last_update_time.elapsed() >= Duration::from_millis(500) {
            let average_delta = last_update_time.elapsed();
            renderer.main_window.window_mut()
                .set_title(format!("RustyBoi - {:.2} FPS", (loop_cycles as f64 / average_delta.as_secs_f64())).as_str());
            last_update_time = Instant::now();
            loop_cycles = 0;
        }
        // Ideally we'd use Instant instead of SDL timer, but for some reason when using Instant
        // we sleep more than we should, leaving us at ~58 fps which causes audio stutters.
        let frame_time = timer.ticks() as i32 - ticks;

        if (!app_state.unbounded || app_state.emulator_paused) && FRAME_DELAY.as_millis() as i32 > frame_time {
            let sleep_time = (FRAME_DELAY.as_millis() as i32 - frame_time) as u64;
            std::thread::sleep(Duration::from_millis(sleep_time));
        }
    }
}

fn handle_events(event: Event, gameboy_runner: &mut GameboyRunner, app_state: &mut AppEmulatorState, renderer: &mut Renderer<ImguiBoi>) -> bool {
    match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        }
        | Event::Window {window_id: 1, win_event: WindowEvent::Close, ..} => {
            gameboy_runner.stop();
            app_state.exit = true;
            return false;
        }
        Event::Window {window_id, win_event: WindowEvent::Close, ..} => {
            renderer.close_immediate_gui();
        }
        Event::DropFile { filename, .. } => {
            if filename.ends_with(".gb") || filename.ends_with(".gbc") {
                debug!("Opening file: {}", filename);
                app_state.awaiting_audio = false;
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
        Event::KeyDown { keycode: Some(key), .. } => {
            if let Some(input_key) = keycode_to_input(key) {
                gameboy_runner.handle_input(input_key, true);
            } else {
                match key {
                    Keycode::LShift => app_state.fast_forward = true,
                    Keycode::P => app_state.emulator_paused = !app_state.emulator_paused,
                    Keycode::K => renderer.setup_immediate_gui("Rustyboi Debugging").unwrap(),
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
        Event::KeyUp { keycode: Some(key), .. } => {
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

struct GameboyRunner {
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
        let emulator = create_emulator(rom_path, options);
        let emulator_thread = std::thread::spawn(move || run_emulator(emulator, frame_sender, response_sender, request_receiver));
        GameboyRunner {
            current_thread: Some(emulator_thread),
            frame_receiver,
            request_sender,
            response_receiver
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
            self.request_sender.send(EmulatorNotification::ExitRequest(Box::new(save_rom)));
            // Since the emulation thread may be blocking trying to send a frame.
            self.frame_receiver.try_recv();
            thread.join();
        }
    }
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

fn run_emulator(mut emulator: Emulator, frame_sender: Sender<[RGB; FRAMEBUFFER_SIZE]>, response_sender: Sender<EmulatorResponse>, notification_receiver: Receiver<EmulatorNotification>) {
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
                    if let Err(e) = response_sender.send(EmulatorResponse::AUDIO(audio_buffer)) {
                        log::error!("Failed to transfer audio buffer due to: {:?}", e);
                        break 'emu_loop;
                    }
                }
                EmulatorNotification::Request(_) => {
                    unimplemented!()
                }
                EmulatorNotification::ExitRequest(save_function) => {
                    save_function(&emulator);
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

fn render_fast(renderer: &mut Renderer<ImguiBoi>, receiver: Receiver<[RGB; FRAMEBUFFER_SIZE]>) {
    loop {
        let res = receiver.recv().unwrap();
        renderer.render_main_window(&res);
    }
}

fn test_fast(cpu_test: &Vec<u8>, sender: Sender<[RGB; FRAMEBUFFER_SIZE]>) {
    let mut emulator = Emulator::new(&cpu_test, EmulatorOptionsBuilder::new()
        .with_mode(CGB)
        .with_display_colours(DEFAULT_DISPLAY_COLOURS)
        .build());

    'mainloop: loop {
            let mut frame_count = 0;
            let start_time = Instant::now();
            loop {
                while frame_count <= 20_000 {
                    if emulator.emulate_cycle() {
                        frame_count += 1;
                        sender.send(*emulator.frame_buffer());
                    }
                }

                if frame_count > 20_000 {
                    println!(
                        "Rendered: {} frames per second after 20_000 frames!",
                        frame_count as f64 / start_time.elapsed().as_secs_f64()
                    );
                    return;
                }
            }
    }
}
