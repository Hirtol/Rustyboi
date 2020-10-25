use log::LevelFilter;
use log::*;
use rustyboi_core::emulator::{Emulator, CYCLES_PER_FRAME};

use rustyboi_core::{InputKey, EmulatorOptionsBuilder};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum::RGB24;
use sdl2::render::{Canvas, Texture, WindowCanvas};
use sdl2::video::Window;
use sdl2::Sdl;
use simplelog::{CombinedLogger, Config, ConfigBuilder, TermLogger, TerminalMode, WriteLogger};

use std::fs::{read, File};

use std::thread::sleep;
use std::time::{Duration, Instant};

use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::event::Event;
use std::io::Write;
use std::ops::Div;
use rustyboi_core::emulator::EmulatorMode::{CGB, DMG};
use rustyboi_core::hardware::ppu::palette::{DisplayColour, RGB};
use image::ImageBuffer;
use image::imageops::FilterType;
use rustyboi::actions::{save_rom, create_emulator};
use crate::sdl::{setup_sdl, fill_texture_and_copy};


mod sdl;

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

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Trace, ConfigBuilder::new().set_location_level(LevelFilter::Off).set_time_level(LevelFilter::Off).set_target_level(LevelFilter::Off).build(), BufWriter::new(File::create("rustyboi.log").unwrap())),
    ])
    .unwrap();

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

    let window = video_subsystem
        .window("RustyBoi", 800, 720)
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().accelerated().build().unwrap();
    let mut screen_texture = setup_sdl(&mut canvas);

    let bootrom_file = read("roms\\cgb_bios.bin").unwrap();

    let cartridge = "roms/Zelda.gb";
    let yellow = "roms/Pokemon - Yellow Version.gbc";
    let _cpu_test = "test roms/blargg_sound/cgb_sound/cgb_sound.gb";
    let _cpu_test2 = "roms/Legend of Zelda, The - Oracle of Seasons (U) [C][!].gbc";

    //let mut emulator = Emulator::new(Option::Some(vec_to_bootrom(&bootrom_file)), &cartridge);

    // test_fast(sdl_context, &mut canvas, &mut screen_texture, &read(cartridge).unwrap());
    //
    // return;

    //TODO: Zelda fix, most likely SOMETHING broken in OAM since sprites are wrong
    //Things to do:
    //1: Overhaul frontend for imgui
    //2: Accuracy improvements to hopefully pass the GBC oracle of seasons. (Sprites?)
    //3: APU improvements to use a proper sampler so that we can re-architect the way we do ticking
    // by doing more lazy evaluation (thus being able to move everything to the scheduler for speed)

    let mut timer = sdl_context.timer().unwrap();
    let emu_opts = EmulatorOptionsBuilder::new()
        .boot_rom(Some(bootrom_file))
        .with_mode(CGB)
        .with_display_colours(KIRBY_DISPLAY_COLOURS)
        .build();

    let mut emulator = create_emulator(_cpu_test, emu_opts);

    let mut cycles = 0;
    let mut loop_cycles = 0;

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut last_update_time: Instant = Instant::now();

    let mut fast_forward = false;
    let mut paused = false;

    audio_queue.resume();

    'mainloop: loop {
        let audio_buffer = emulator.audio_buffer();

        // Temporary hack to make audio not buffer up while fast forwarding,
        // in the future could consider downsampling the sped up audio for a cool effect.
        if !fast_forward {
            audio_queue.queue(audio_buffer);
        } else {
            audio_queue.clear();
        }
        emulator.clear_audio_buffer();

        let ticks = timer.ticks() as i32;

        for event in event_pump.poll_iter() {
            if !handle_events(event, &mut emulator, &mut fast_forward, &mut paused) {
                break 'mainloop;
            }
        }

        let cycle_count_to_reach = if fast_forward { FAST_FORWARD_MULTIPLIER } else { 1 };

        // 30-40k seems to be a decent spot for audio syncing.
        // I should really figure out proper audio syncing ._.
        if audio_queue.size() < 50_000 {
            while cycles < cycle_count_to_reach && !paused {
                if let (_, true) = emulator.emulate_cycle() {
                    cycles += 1;
                }
                if (emulator.audio_buffer().len() >= 1550) && !fast_forward {
                    break;
                }
            }

            cycles = 0;
        }

        fill_texture_and_copy(
            &mut canvas,
            &mut screen_texture,
            emulator.frame_buffer(),
        );

        canvas.present();

        let frame_time = timer.ticks() as i32;

        let frame_time = frame_time - ticks;

        if FRAME_DELAY.as_millis() as i32 > frame_time {
            let sleeptime = (FRAME_DELAY.as_millis() as i32 - frame_time) as u64;
            std::thread::sleep(Duration::from_millis(sleeptime));
        }

        loop_cycles += 1;

        if loop_cycles == 10 {
            let average_delta = last_update_time.elapsed().div(10);
            loop_cycles = 0;
            canvas
                .window_mut()
                .set_title(format!("RustyBoi - {:.2} FPS", (1.0 / average_delta.as_secs_f64() * (if fast_forward { FAST_FORWARD_MULTIPLIER } else { 1 } as f64))).as_str());
            last_update_time = Instant::now();
        }
    }
}

fn handle_events(event: Event, emulator: &mut Emulator, fast_forward: &mut bool, pause: &mut bool) -> bool {
    match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => {
            save_rom(emulator);
            return false;
        }
        Event::DropFile { filename, .. } => {
            if filename.ends_with(".gb") || filename.ends_with(".gbc") {
                debug!("Opening file: {}", filename);
                save_rom(emulator);
                let emu_opts = EmulatorOptionsBuilder::new()
                    .with_mode(CGB)
                    .with_display_colours(KIRBY_DISPLAY_COLOURS)
                    .build();
                *emulator = create_emulator(&filename, emu_opts);
            } else {
                warn!("Attempted opening of file: {} which is not a GameBoy rom!", filename);
            }
        }
        Event::KeyDown { keycode: Some(key), .. } => {
            if let Some(input_key) = keycode_to_input(key) {
                emulator.handle_input(input_key, true);
            } else {
                match key {
                    Keycode::LShift => *fast_forward = true,
                    Keycode::P => *pause = !*pause,
                    Keycode::O => println!("{:#?}", emulator.oam()),
                    Keycode::L => {
                        let mut true_image_buffer = vec![0u8; 768*8*8*3];

                        for (i, colour) in emulator.vram_tiles().iter().enumerate() {
                            let offset = i * 3;
                            true_image_buffer[offset] = colour.0;
                            true_image_buffer[offset + 1] = colour.1;
                            true_image_buffer[offset + 2] = colour.2;
                        }
                        let temp_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
                            image::ImageBuffer::from_raw(128, 384, true_image_buffer).unwrap();
                        let temp_buffer = image::imageops::resize(&temp_buffer, 256, 768, FilterType::Nearest);
                        temp_buffer
                            .save(format!("vram_dump.png"))
                            .unwrap();
                    }
                    _ => {}
                }
            }
        }
        Event::KeyUp { keycode: Some(key), .. } => {
            if let Some(input_key) = keycode_to_input(key) {
                emulator.handle_input(input_key, false);
            } else {
                match key {
                    Keycode::LShift => *fast_forward = false,
                    _ => {}
                }
            }
        }
        _ => {}
    }

    true
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

fn test_fast(sdl_context: Sdl, mut canvas: &mut Canvas<Window>, mut screen_texture: &mut Texture, cpu_test: &Vec<u8>) {
    let mut emulator = Emulator::new(&cpu_test, EmulatorOptionsBuilder::new()
        .with_mode(DMG)
        .with_display_colours(DEFAULT_DISPLAY_COLOURS)
        .build());
    let _count: u128 = 0;

    let mut event_pump = sdl_context.event_pump().unwrap();

    let start_time = Instant::now();

    let mut last_update_time: Instant = Instant::now();
    let mut delta_time: Duration = Duration::default();

    'mainloop: loop {
        let frame_start = Instant::now();

        delta_time = frame_start.duration_since(last_update_time);

        let to_sleep = FRAME_DELAY.checked_sub(delta_time);

        if let Some(to_sleep) = to_sleep {
            sleep(to_sleep);
        }

        for event in event_pump.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'mainloop;
                }
                Event::DropFile { filename, .. } => {
                    debug!("Dropped file: {}", filename);
                }
                _ => {}
            }
        }

        if start_time.elapsed() < Duration::new(1, 0) {
            let mut frame_count = 0;
            let start_time = Instant::now();
            loop {
                while frame_count <= 20_000 {
                    if emulator.emulate_cycle().1 {
                        frame_count += 1;
                    }
                }

                if frame_count > 20_000 {
                    println!(
                        "Rendered: {} frames per second after 20_000 frames!",
                        frame_count as f64 / start_time.elapsed().as_secs_f64()
                    );
                    break;
                }
            }
        }

        fill_texture_and_copy(
            &mut canvas,
            &mut screen_texture, emulator.frame_buffer(),
        );

        canvas.present();

        canvas
            .window_mut()
            .set_title(format!("RustyBoi - {} FPS", 1.0 / last_update_time.elapsed().as_secs_f64()).as_str());
        last_update_time = frame_start;
    }
}
