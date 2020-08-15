use log::trace;
use log::LevelFilter;
use rustyboi_core::hardware::*;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Warn, Config::default(), File::create("my_rust_binary.log").unwrap()),
    ])
    .unwrap();

    println!("Hello");
    trace!("Hello World!");
}
