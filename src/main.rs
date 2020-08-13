use simplelog::{CombinedLogger, TermLogger, WriteLogger, Config, TerminalMode};
use log::LevelFilter;
use log::trace;
use rustyboi_core::hardware::*;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Warn, Config::default(), File::create("my_rust_binary.log").unwrap()),
    ]).unwrap();

    println!("Hello");
    trace!("Hello World!");
}
