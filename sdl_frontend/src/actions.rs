use directories::ProjectDirs;
use rustyboi_core::gb_emu::GameBoyEmulator;
use rustyboi_core::hardware::cartridge::header::CartridgeHeader;

use rustyboi_core::{EmulatorOptions, EmulatorOptionsBuilder};
use std::fs::{create_dir_all, read, File};
use std::io::Write;
use std::path::Path;

/// Function to call in order to save external ram (in case it's present)
/// as well as any additional cleanup as required.
pub fn save_rom(emulator: &GameBoyEmulator) {
    if let Some(ram) = emulator.battery_ram() {
        let save_dir = ProjectDirs::from("", "Hirtol", "Rustyboi")
            .expect("Could not get access to data dir for saving!")
            .data_dir()
            .join("saves");
        create_dir_all(&save_dir);
        // Really, this expect case shouldn't ever be reached.
        let title = emulator.game_title().expect("No cartridge loaded, can't save!").trim();

        let mut save_file =
            File::create(save_dir.join(format!("{}.save", title))).expect("Could not create the save file");
        save_file.write(ram);

        log::debug!(
            "Finished saving the external ram with size: {} successfully!",
            ram.len()
        );
    }
}

/// Create an emulator for the ROM provided by `rom_path`.
/// In case the file provided is not a rom the program will *probably* crash.
///
/// Any external ram will also automatically be loaded if present.
pub fn create_emulator(rom_path: impl AsRef<Path>, options: EmulatorOptions) -> GameBoyEmulator {
    let rom = read(rom_path.as_ref()).expect(&format!("Could not open ROM file {:?}!", rom_path.as_ref()));
    let saved_ram = find_saved_ram(find_rom_name(&rom));

    log::info!(
        "Created emulator for Path {:?} with saved data: {}",
        rom_path.as_ref(),
        saved_ram.is_some()
    );

    let emu_options = EmulatorOptionsBuilder::from(options).with_saved_ram(saved_ram).build();

    GameBoyEmulator::new(&rom, emu_options)
}

pub fn find_saved_ram(name: impl AsRef<str>) -> Option<Vec<u8>> {
    let save_dir = ProjectDirs::from("", "Hirtol", "Rustyboi")
        .expect("Could not get access to data dir for saving!")
        .data_dir()
        .join("saves");
    create_dir_all(&save_dir);

    read(save_dir.join(format!("{}.save", name.as_ref()))).ok()
}

pub fn find_rom_name(rom: &[u8]) -> String {
    CartridgeHeader::new(rom).title.trim().to_owned()
}
