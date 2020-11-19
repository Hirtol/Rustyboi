use std::fs::read;
use std::path::Path;

use criterion::{BenchmarkGroup, Criterion, criterion_group, criterion_main};
use criterion::measurement::WallTime;
use criterion_cycles_per_byte::CyclesPerByte;

use rustyboi_core::emulator::Emulator;
use rustyboi_core::emulator::GameBoyModel::CGB;
use rustyboi_core::EmulatorOptionsBuilder;

fn emulator_benchmark(c: &mut Criterion) {
    let rom_data = read("..\\roms\\Zelda.gb").unwrap();

    let mut emulator = Emulator::new(&rom_data, EmulatorOptionsBuilder::new().build());
    c.bench_function("Emulate to Vblank", |b| b.iter(|| emulator.run_to_vblank()));
    let mut emulator = Emulator::new(&rom_data, EmulatorOptionsBuilder::new().build());
    c.bench_function("Emulate Cycle", |b| b.iter(|| emulator.emulate_cycle()));
}

fn ppu_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("PPU Benches");

    run_scanline_benchmark("..\\roms\\Zelda.gb", &mut group, "Zelda");
    run_scanline_benchmark("..\\roms\\Kirby's Dream Land.gb", &mut group, "Kirby");
    run_scanline_benchmark("..\\roms\\Pokemon Red.gb", &mut group, "Pokemon Red");
    run_scanline_benchmark("..\\roms\\Prehistorik Man (U).gb", &mut group, "Prehistorik Man");
    run_scanline_benchmark("..\\roms\\Tomb Raider - Curse of the Sword (U) [C][!].gbc", &mut group, "Tomb Raider");
    run_scanline_benchmark("..\\roms\\Zelda.gbc", &mut group, "Zelda GBC");

    group.finish();
}

fn run_scanline_benchmark(path: impl AsRef<Path>, group: &mut BenchmarkGroup<WallTime>, name: impl AsRef<str>) {
    let rom_data = read(path.as_ref()).unwrap();
    let is_cgb_rom = path.as_ref().extension().unwrap_or_default() == "gbc";

    let mut emulator = if is_cgb_rom {
        Emulator::new(&rom_data, EmulatorOptionsBuilder::new().with_mode(CGB).build())
    } else {
        Emulator::new(&rom_data, EmulatorOptionsBuilder::new().build())
    };

    for _ in 0..40 {
        emulator.run_to_vblank();
    }

    emulator.ppu().current_y = 0;
    group.bench_function(format!("Benchmark {} full framebuffer", name.as_ref()), |b| b.iter(|| {
        if is_cgb_rom {
            emulator.ppu().draw_cgb_scanline();
        } else {
            emulator.ppu().draw_scanline();
        }
        emulator.ppu().current_y = (emulator.ppu().current_y % 143) + 1;
    }));
}

criterion_group!(benches, emulator_benchmark);

// criterion_group!(
//     name = ppu_benches;
//     config = Criterion::default().with_measurement(CyclesPerByte);
//     targets = emulator_benchmark
// );

criterion_main!(benches);
