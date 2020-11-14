use rustyboi_core::hardware::ppu::FRAMEBUFFER_SIZE;
use rustyboi_core::hardware::ppu::palette::RGB;
use crossbeam::channel::*;
use crate::rendering::Renderer;
use rustyboi_core::emulator::Emulator;
use rustyboi_core::{EmulatorOptionsBuilder, EmulatorOptions};
use rustyboi_core::emulator::EmulatorMode::CGB;
use crate::DEFAULT_DISPLAY_COLOURS;
use std::time::Instant;
use std::fs::read;
use std::path::Path;
use crate::rendering::immediate::ImmediateGui;
use crate::options::AppOptions;
use std::process::exit;

#[inline(always)]
pub fn run_benchmark(options: &AppOptions) {
    if options.benchmark {
        let benchmarking_opts = EmulatorOptionsBuilder::new()
            .with_mode(CGB)
            .with_display_colour(DEFAULT_DISPLAY_COLOURS)
            .build();
        Benchmarking::benchmark_without_render(&options.rom_path, benchmarking_opts);
        exit(0);
    }
}

pub struct Benchmarking;

impl Benchmarking {
    #[inline(always)]
    pub fn benchmark_with_render<T: ImmediateGui>(cartridge: impl AsRef<Path>, renderer: &mut Renderer<T>, emu_opts: EmulatorOptions) {
        let (frame_sender, frame_receiver) = bounded(1);
        let data = read(cartridge).unwrap();
        std::thread::spawn(move || run_with_send(&data, frame_sender, emu_opts));

        loop {
            if let Ok(res) = frame_receiver.recv() {
                renderer.render_main_window(&res);
            } else {
                return;
            }
        }
    }

    #[inline(always)]
    pub fn benchmark_without_render(cartridge: impl AsRef<Path>, emu_opts: EmulatorOptions) {
        let mut emulator = Emulator::new(&read(cartridge).unwrap(), emu_opts);

        'mainloop: loop {
            let mut frame_count = 0;
            let start_time = Instant::now();
            loop {
                while frame_count <= 20_000 {
                    emulator.run_to_vblank();
                    frame_count += 1;
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
}

fn run_with_send(cartridge: &Vec<u8>, sender: Sender<[RGB; FRAMEBUFFER_SIZE]>, emu_opts: EmulatorOptions) {
    let mut emulator = Emulator::new(cartridge, emu_opts);

    'mainloop: loop {
        let mut frame_count = 0;
        let start_time = Instant::now();
        loop {
            while frame_count <= 20_000 {
                emulator.run_to_vblank();
                frame_count += 1;
                sender.send(*emulator.frame_buffer());
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