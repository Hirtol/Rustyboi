use clap::Clap;

#[derive(Clap)]
#[clap(version = "1.0", author = "Hirtol")]
pub struct Options {
    /// The path to the folder with all Blargg tests.
    #[clap(short, default_value = "test roms/blargg/")]
    pub blargg_path: String,
    /// The path to the folder with all MoonEyeGB tests.
    #[clap(short, default_value = "test roms/mooneye/")]
    pub mooneye_path: String,
}