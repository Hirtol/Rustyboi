use criterion::{criterion_group, criterion_main, Criterion};
use rustyboi_core::emulator::Emulator;
use rustyboi_core::EmulatorOptionsBuilder;
use std::fs::read;
use criterion_cycles_per_byte::CyclesPerByte;

fn emulator_benchmark(c: &mut Criterion) {
    let cpu_test = read("..\\roms\\Zelda.gb").unwrap();

    let mut emulator = Emulator::new(&cpu_test, EmulatorOptionsBuilder::new().build());
    c.bench_function("Emulate Cycle", |b| b.iter(|| emulator.emulate_cycle()));
}

fn ppu_benchmark(c: &mut Criterion) {
    let cpu_test = read("..\\roms\\Zelda.gb").unwrap();

    let mut emulator = Emulator::new(&cpu_test, EmulatorOptionsBuilder::new().build());
    let mut group = c.benchmark_group("PPU Benches");

    emulator.ppu().current_y = 0;
    group.bench_function("Benchmark empty framebuffer", |b| b.iter(|| {
        emulator.ppu().draw_scanline();
        emulator.ppu().current_y = (emulator.ppu().current_y % 143) + 1;
    }));

    for _ in 0..40 {
        emulator.run_to_vblank();
    }

    emulator.ppu().current_y = 0;
    group.bench_function("Benchmark full framebuffer", |b| b.iter(|| {
        emulator.ppu().draw_scanline();
        emulator.ppu().current_y = (emulator.ppu().current_y % 143) + 1;
    }));

    group.finish();
}

fn benchmark() {}

criterion_group!(benches, emulator_benchmark, ppu_benchmark);

// criterion_group!(
//     name = ppu_benches;
//     config = Criterion::default().with_measurement(CyclesPerByte);
//     targets = emulator_benchmark
// );

criterion_main!(benches);
