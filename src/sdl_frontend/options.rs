use gumdrop::Options;

#[derive(Options, Debug, Default)]
pub struct AppOptions {
    /// Print this help message
    #[options()]
    help: bool,
    /// The path to the rom which you want to run
    #[options(default = "roms/Zelda.gb")]
    pub rom_path: String,
    /// The path to the DMG bootrom
    #[options(default = "roms/DMG_ROM.bin")]
    pub dmg_boot_rom: String,
    /// The path to the CGB bootrom
    #[options(default = "roms/cgb_bios.bin")]
    pub cgb_boot_rom: String,
    /// If provided will run a benchmark on the provided rom, and then exit.
    #[options()]
    pub benchmark: bool,
}
