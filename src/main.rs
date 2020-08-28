use log::LevelFilter;
use log::*;
use rustyboi_core::emulator::Emulator;
use rustyboi_core::hardware::cartridge::Cartridge;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger, ConfigBuilder};
use std::convert::TryInto;
use std::fs::{read, File};
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::io::BufWriter;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Trace, ConfigBuilder::new().set_location_level(LevelFilter::Off).set_time_level(LevelFilter::Off).set_target_level(LevelFilter::Off).build(), BufWriter::new(File::create("my_rust_binary.log").unwrap())),
    ])
    .unwrap();

    let bootrom_file = read(
        "***REMOVED***roms\\DMG_ROM.bin",
    )
    .unwrap();

    let mut cartridge =
        read("***REMOVED***roms\\Tetris.gb")
            .unwrap();
    let cpu_test = read("***REMOVED***test roms\\cpu_instrs\\individual\\02-interrupts.gb").unwrap();

    //let mut emulator = Emulator::new(Option::Some(vec_to_bootrom(bootrom_file)), &cartridge);
    let mut emulator = Emulator::new(Option::None, &cpu_test);
    let mut count: u128 = 0;

    let start_time = Instant::now();

    loop {
        emulator.emulate_cycle();
        count += 1;
        if count % 100_000_000 == 0 {
            warn!("REACHED VALUE: {} AFTER: {:?}", count, start_time.elapsed());
            break;
        }
        //sleep(Duration::from_millis(10));
    }
}

fn vec_to_bootrom(vec: Vec<u8>) -> [u8; 256] {
    let mut result = [0_u8; 256];

    for (i, instr) in vec.iter().enumerate() {
        result[i] = *instr;
    }

    result
}
