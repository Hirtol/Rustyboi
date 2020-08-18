use log::trace;
use log::LevelFilter;
use rustyboi_core::hardware::cpu::*;
use rustyboi_core::hardware::*;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};
use rustyboi_core::hardware::cartridge::Cartridge;
use std::fs::read;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Warn, Config::default(), File::create("my_rust_binary.log").unwrap()),
    ])
    .unwrap();
    let mut cpu = CPU::new();
    cpu.step_cycle();
    trace!("Hello World!");

    let mut cartridge = Cartridge::new(&read("***REMOVED***roms\\Tetris.gb").unwrap());

    println!("{:?}", cartridge);
}
