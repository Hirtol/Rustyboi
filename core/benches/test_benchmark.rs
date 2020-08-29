use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustyboi_core::emulator::Emulator;
use rustyboi_core::hardware::ppu::palette::{DisplayColour, RGB};
use std::fs::read;

fn emulator_benchmark(c: &mut Criterion) {
    let mut cpu_test = read("***REMOVED***test roms\\cpu_instrs\\individual\\06-ld r,r.gb").unwrap();
    let display_colors = DisplayColour {
        white: RGB(155, 188, 15),
        light_grey: RGB(139, 172, 15),
        dark_grey: RGB(48, 98, 48),
        black: RGB(15, 56, 15),
    };

    let mut emulator = Emulator::new(Option::None, &cpu_test, display_colors);
    c.bench_function("Emulate Cycle", |b| b.iter(|| emulator.emulate_cycle()));
}

criterion_group!(benches, emulator_benchmark);

criterion_main!(benches);
