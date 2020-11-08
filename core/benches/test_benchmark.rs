use criterion::{criterion_group, criterion_main, Criterion};
use rustyboi_core::emulator::Emulator;
use rustyboi_core::EmulatorOptionsBuilder;
use std::fs::read;

fn emulator_benchmark(c: &mut Criterion) {
    let cpu_test = read("..\\roms\\Zelda.gb").unwrap();

    let mut emulator = Emulator::new(&cpu_test, EmulatorOptionsBuilder::new().build());
    c.bench_function("Emulate Cycle", |b| b.iter(|| emulator.emulate_cycle()));
}

criterion_group!(benches, emulator_benchmark);

criterion_main!(benches);
