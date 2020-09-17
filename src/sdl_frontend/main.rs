use log::LevelFilter;
use log::*;
use rustyboi_core::emulator::{Emulator, CYCLES_PER_FRAME};

use rustyboi_core::{emulator, DmgColor, InputKey};
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

use crate::display::{DisplayColour, RGB};
use sdl2::event::Event;
use std::io::BufWriter;
use std::ops::Div;

mod display;

const KIRBY_DISPLAY_COLOURS: DisplayColour = DisplayColour {
    white: RGB(44, 44, 150),
    light_grey: RGB(119, 51, 231),
    dark_grey: RGB(231, 134, 134),
    black: RGB(247, 190, 247),
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

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Trace, ConfigBuilder::new().set_location_level(LevelFilter::Off).set_time_level(LevelFilter::Off).set_target_level(LevelFilter::Off).build(), BufWriter::new(File::create("rustyboi.log").unwrap())),
    ])
    .unwrap();

    let sdl_context = sdl2::init().expect("Failed to initialise SDL context!");
    let video_subsystem = sdl_context
        .video()
        .expect("SDL context failed to initialise video!");

    let window = video_subsystem
        .window("RustyBoi", 800, 720)
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().accelerated().build().unwrap();
    let mut screen_texture = setup_sdl(&mut canvas);

    let bootrom_file = read("roms\\DMG_ROM.bin").unwrap();

    let cartridge = read("roms\\Zelda.gb").unwrap();
    let cpu_test = read("test roms/blargg/cpu_instrs/individual/02-interrupts.gb").unwrap();
    let cpu_test2 = read("test roms/mooneye/tests/emulator-only/mbc1/mbc1_ram_64kb.gb").unwrap();

    //let mut emulator = Emulator::new(Option::Some(vec_to_bootrom(&bootrom_file)), &cartridge);

    // test_fast(sdl_context, &mut canvas, &mut screen_texture, &cpu_test);
    //
    // return;

    let mut timer = sdl_context.timer().unwrap();

    let mut emulator = Emulator::new(Option::None, &cartridge);

    let mut cycles = 0;
    let mut loop_cycles = 0;
    let mut delta_acc = Duration::new(0, 0);

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut last_update_time: Instant = Instant::now();

    'mainloop: loop {
        let frame_start = Instant::now();
        let ticks = timer.ticks() as i32;

        for event in event_pump.poll_iter() {
            if !handle_events(event, &mut emulator) {
                break 'mainloop;
            }
        }
        // Emulate exactly one frame's worth.
        while cycles < CYCLES_PER_FRAME {
            cycles += emulator.emulate_cycle() as u32;
        }

        cycles -= CYCLES_PER_FRAME;

        fill_texture_and_copy(
            &mut canvas,
            &mut screen_texture,
            &emulator.frame_buffer(),
            &DEFAULT_DISPLAY_COLOURS,
        );

        canvas.present();

        let frame_time = timer.ticks() as i32;

        let frame_time = frame_time - ticks;

        if FRAME_DELAY.as_millis() as i32 > frame_time {
            let sleeptime = (FRAME_DELAY.as_millis() as i32 - frame_time) as u64;
            delta_acc +=
                Duration::from_millis((FRAME_DELAY.as_millis() as i32 - frame_time) as u64);
            std::thread::sleep(Duration::from_millis(sleeptime));
        }

        loop_cycles += 1;

        if loop_cycles == 10 {
            let average_delta = delta_acc.div(9);
            loop_cycles = 0;
            delta_acc = Duration::default();
            canvas.window_mut().set_title(
                format!("RustyBoi - {:.2} FPS", 1.0 / average_delta.as_secs_f64()).as_str(),
            );
        }

        last_update_time = frame_start;
    }
}

fn handle_events(event: Event, emulator: &mut Emulator) -> bool {
    match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => {
            return false;
        }
        Event::DropFile { filename, .. } => {
            if filename.ends_with(".gb") {
                debug!("Opening file: {}", filename);
                let new_cartridge = read(filename).expect("Could not open the provided file!");
                *emulator = Emulator::new(Option::None, &new_cartridge);
            } else {
                warn!(
                    "Attempted opening of file: {} which is not a GameBoy rom!",
                    filename
                );
            }
        }
        Event::KeyDown {
            keycode: Some(key), ..
        } => {
            if let Some(input_key) = keycode_to_input(key) {
                emulator.handle_input(input_key, true);
            }
        }
        Event::KeyUp {
            keycode: Some(key), ..
        } => {
            if let Some(input_key) = keycode_to_input(key) {
                emulator.handle_input(input_key, false);
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

fn setup_sdl(canvas: &mut WindowCanvas) -> Texture {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    // Ensure aspect ratio is kept, in the future we could change this if we want more GUI elements.
    // Or just render ImGui on top ㄟ( ▔, ▔ )ㄏ
    canvas.set_logical_size(160, 144);
    canvas.set_scale(1.0, 1.0);

    canvas.present();
    canvas.create_texture_streaming(RGB24, 160, 144).unwrap()
}

/// This function assumes pixel_buffer size * 3 == texture buffer size, otherwise panic
fn fill_texture_and_copy(
    canvas: &mut WindowCanvas,
    texture: &mut Texture,
    pixel_buffer: &[DmgColor],
    colorizer: &DisplayColour,
) {
    texture.with_lock(Option::None, |arr, _pitch| {
        for (i, colour) in pixel_buffer.iter().enumerate() {
            let colour = colorizer.get_color(colour);
            let offset = i * 3;
            arr[offset] = colour.0;
            arr[offset + 1] = colour.1;
            arr[offset + 2] = colour.2;
        }
    });
    canvas.copy(&texture, None, None);
}

fn vec_to_bootrom(vec: &Vec<u8>) -> [u8; 256] {
    let mut result = [0u8; 256];

    for (i, instr) in vec.iter().enumerate() {
        result[i] = *instr;
    }

    result
}

fn test_fast(
    sdl_context: Sdl,
    mut canvas: &mut Canvas<Window>,
    mut screen_texture: &mut Texture,
    cpu_test: &Vec<u8>,
) {
    let mut emulator = Emulator::new(Option::None, &cpu_test);
    let mut count: u128 = 0;

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

        // Temp loop for testing.
        // TODO: Implement actual cycling.
        if start_time.elapsed() < Duration::new(1, 0) {
            loop {
                emulator.emulate_cycle();
                count += 1;
                if count % 100_000_000 == 0 {
                    warn!("REACHED VALUE: {} AFTER: {:?}", count, start_time.elapsed());
                    break;
                }
            }
        }

        fill_texture_and_copy(
            &mut canvas,
            &mut screen_texture,
            &emulator.frame_buffer(),
            &DEFAULT_DISPLAY_COLOURS,
        );

        canvas.present();

        canvas.window_mut().set_title(
            format!(
                "RustyBoi - {} FPS",
                1.0 / last_update_time.elapsed().as_secs_f64()
            )
            .as_str(),
        );
        last_update_time = frame_start;
    }
}