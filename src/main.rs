use log::LevelFilter;
use log::*;
use rustyboi_core::emulator::{Emulator, CYCLES_PER_FRAME};
use rustyboi_core::hardware::cartridge::Cartridge;
use rustyboi_core::hardware::ppu::palette::{DisplayColour, RGB};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum::RGB24;
use sdl2::render::{Texture, WindowCanvas, Canvas};
use simplelog::{CombinedLogger, Config, ConfigBuilder, TermLogger, TerminalMode, WriteLogger};
use std::convert::TryInto;
use std::fs::{read, File};
use std::io::BufWriter;
use std::thread::sleep;
use std::time::{Duration, Instant};
use sdl2::Sdl;
use sdl2::video::Window;

const DISPLAY_COLOURS: DisplayColour = DisplayColour {
    white: RGB(155, 188, 15),
    light_grey: RGB(139, 172, 15),
    dark_grey: RGB(48, 98, 48),
    black: RGB(15, 56, 15),
};

const FPS: u64 = 60;
const FRAME_DELAY: Duration = Duration::from_nanos(1_000_000_000u64 / FPS);

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Trace, ConfigBuilder::new().set_location_level(LevelFilter::Off).set_time_level(LevelFilter::Off).set_target_level(LevelFilter::Off).build(), BufWriter::new(File::create("my_rust_binary.log").unwrap())),
    ])
    .unwrap();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let mut window = video_subsystem
        .window("RustyBoi", 800, 720)
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().accelerated().build().unwrap();

    let mut screen_texture = setup_sdl(&mut canvas);

    let bootrom_file = read(
        "***REMOVED***roms\\DMG_ROM.bin",
    )
    .unwrap();

    let mut cartridge =
        read("***REMOVED***roms\\Tetris.gb")
            .unwrap();
    let cpu_test = read("***REMOVED***test roms\\cpu_instrs\\individual\\07-jr,jp,call,ret,rst.gb").unwrap();

    //let mut emulator = Emulator::new(Option::Some(vec_to_bootrom(bootrom_file)), &cartridge);

    // test_fast(sdl_context, &mut canvas, &mut screen_texture, &cpu_test);
    //
    // return;

    let mut emulator = Emulator::new(Option::None, &cpu_test, DISPLAY_COLOURS);
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

        for i in 0..CYCLES_PER_FRAME {
            emulator.emulate_cycle();
        }

        fill_texture_and_copy(&mut canvas, &mut screen_texture, &emulator.frame_buffer());

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

fn test_fast(sdl_context: Sdl, mut canvas: &mut Canvas<Window>, mut screen_texture: &mut Texture, cpu_test: &Vec<u8>) {
    let mut emulator = Emulator::new(Option::None, &cpu_test, DISPLAY_COLOURS);
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
                    emulator.tilemap_image();
                    warn!("REACHED VALUE: {} AFTER: {:?}", count, start_time.elapsed());
                    break;
                }
            }
        }

        fill_texture_and_copy(&mut canvas, &mut screen_texture, &emulator.frame_buffer());

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

fn setup_sdl(canvas: &mut WindowCanvas) -> Texture {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    // Ensure aspect ratio is kept, in the future we could change this if we want.
    // Or just render imgui on top ㄟ( ▔, ▔ )ㄏ
    canvas.set_logical_size(160, 144);
    canvas.set_scale(1.0, 1.0);

    canvas.present();
    canvas.create_texture_streaming(RGB24, 160, 144).unwrap()
}

/// This function assumes pixel_buffer size == texture buffer size, otherwise panic :D
fn fill_texture_and_copy(canvas: &mut WindowCanvas, texture: &mut Texture, pixel_buffer: &[u8]) {
    texture.with_lock(Option::None, |arr, pitch| {
        arr.copy_from_slice(&pixel_buffer)
    });
    canvas.copy(&texture, None, None);
}

fn vec_to_bootrom(vec: Vec<u8>) -> [u8; 256] {
    let mut result = [0_u8; 256];

    for (i, instr) in vec.iter().enumerate() {
        result[i] = *instr;
    }

    result
}
