use log::trace;
use log::LevelFilter;
use rustyboi_core::hardware::cpu::*;
use rustyboi_core::hardware::*;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};
use rustyboi_core::hardware::cartridge::Cartridge;
use std::fs::read;
use std::convert::TryInto;
use rustyboi_core::emulator::Emulator;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Warn, Config::default(), File::create("my_rust_binary.log").unwrap()),
    ]).unwrap();

    let bootrom_file = read("C:\\Users\\Valentijn\\Desktop\\Rust\\Rustyboi\\roms\\DMG_ROM.bin").unwrap();

    let mut cartridge = read("C:\\Users\\Valentijn\\Desktop\\Rust\\Rustyboi\\roms\\Tetris.gb").unwrap();

    let mut emulator = Emulator::new(Option::Some(vec_to_bootrom(bootrom_file)), &cartridge);

    loop {
        emulator.emulate_cycle();
    }

}


fn vec_to_bootrom(vec: Vec<u8>) -> [u8; 256]{
    let mut result = [0_u8; 256];

    for (i, instr) in vec.iter().enumerate() {
        result[i] = *instr;
    }

    result
}