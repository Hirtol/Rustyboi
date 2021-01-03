use gumdrop::Options;

#[derive(Options)]
pub struct AppOptions {
    /// Print this help message
    #[options()]
    help: bool,
    /// The path to the folder with all Blargg tests.
    #[options(default = "test roms/auto-run/")]
    pub test_path: String,
    #[options(default = "testing_frames/")]
    pub output_path: String,
    /// The path to the DMG bootrom
    #[options(default = "roms/DMG_ROM.bin")]
    pub dmg_boot_rom: String,
    /// The path to the CGB bootrom
    #[options(default = "roms/cgb_bios.bin")]
    pub cgb_boot_rom: String,
}
