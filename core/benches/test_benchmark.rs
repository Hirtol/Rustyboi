use criterion::{criterion_group, criterion_main, Criterion};
use rustyboi_core::emulator::Emulator;
use std::fs::read;

fn emulator_benchmark(c: &mut Criterion) {
    let cpu_test = read("..\\test roms\\blargg\\cpu_instrs\\individual\\06-ld r,r.gb").unwrap();

    let mut emulator = Emulator::new(Option::None, &cpu_test);
    c.bench_function("Emulate Cycle", |b| b.iter(|| emulator.emulate_cycle()));
}

criterion_group!(benches, emulator_benchmark);

criterion_main!(benches);
