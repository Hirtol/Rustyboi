use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::read;
use rustyboi_core::emulator::Emulator;

fn emulator_benchmark(c: &mut Criterion) {
    let mut cpu_test = read("***REMOVED***test roms\\cpu_instrs\\individual\\06-ld r,r.gb").unwrap();

    let mut emulator = Emulator::new(Option::None, &cpu_test);
    c.bench_function("Emulate Cycle", |b| b.iter(|| emulator.emulate_cycle()));
}

criterion_group!(benches, emulator_benchmark);

criterion_main!(benches);