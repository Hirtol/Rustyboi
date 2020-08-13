use simplelog::{CombinedLogger, TermLogger, WriteLogger, Config, TerminalMode};
use log::LevelFilter;


fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed),
        //WriteLogger::new(LevelFilter::Warn, Config::default(), File::create("my_rust_binary.log").unwrap()),
    ]).unwrap();
}
