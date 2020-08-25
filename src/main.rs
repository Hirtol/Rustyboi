use log::LevelFilter;
use log::*;
use rustyboi_core::emulator::Emulator;
use rustyboi_core::hardware::cartridge::Cartridge;
use rustyboi_core::hardware::cpu::*;
use rustyboi_core::hardware::*;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};
use std::convert::TryInto;
use std::fs::read;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Warn, Config::default(), File::create("my_rust_binary.log").unwrap()),
    ])
    .unwrap();

    let bootrom_file = read(
        "***REMOVED***roms\\DMG_ROM.bin",
    )
    .unwrap();

    let mut cartridge =
        read("***REMOVED***roms\\Tetris.gb")
            .unwrap();
    let mut cpu_test = read("***REMOVED***test roms\\cpu_instrs\\individual\\03-op sp,hl.gb").unwrap();

    //let mut emulator = Emulator::new(Option::Some(vec_to_bootrom(bootrom_file)), &cartridge);
    let mut emulator = Emulator::new(Option::None, &cpu_test);
    let mut count: u128 = 0;
    loop {
        emulator.emulate_cycle();
        count += 1;
        if count % 1_000_000 == 0 {
            warn!("REACHED VALUE: {}", count);
        }
        //sleep(Duration::from_millis(10));
    }
}

fn vec_to_bootrom(vec: Vec<u8>) -> [u8; 256] {
    let mut result = [0_u8; 256];

    for (i, instr) in vec.iter().enumerate() {
        debug!("Writing to bootrom byte: {:02X}", instr);
        result[i] = *instr;
    }

    result
}
