use log::trace;
use log::LevelFilter;
use rustyboi_core::hardware::cpu::*;
use rustyboi_core::hardware::*;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};
use rustyboi_core::hardware::cartridge::Cartridge;
use std::fs::read;
use std::convert::TryInto;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Warn, Config::default(), File::create("my_rust_binary.log").unwrap()),
    ])
    .unwrap();

    let bootrom_file = read("C:\\Users\\Valentijn\\Desktop\\Rust\\Rustyboi\\roms\\DMG_ROM.bin").unwrap();

    let mut cpu = CPU::new(Option::None, &vec![0u8]);
    cpu.step_cycle();
    trace!("Hello World!");

    let mut cartridge = Cartridge::new(&read("***REMOVED***roms\\Tetris.gb").unwrap());

    println!("{:?}", cartridge);
}
