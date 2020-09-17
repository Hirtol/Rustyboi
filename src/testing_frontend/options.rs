use gumdrop::Options;

#[derive(Options)]
pub struct AppOptions {
    /// Print this help message
    #[options()]
    help: bool,
    /// The path to the folder with all Blargg tests.
    #[options(default = "test roms/blargg/")]
    pub blargg_path: String,
    /// The path to the folder with all MoonEyeGB tests.
    #[options(default = "test roms/mooneye/")]
    pub mooneye_path: String,
}
