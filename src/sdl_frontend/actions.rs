use directories::ProjectDirs;
use rustyboi_core::emulator::Emulator;
use std::fs::{create_dir_all, read, File};
use std::io::Write;
use std::path::Path;
use rustyboi_core::hardware::cartridge::Cartridge;

/// Function to call in order to save external ram (in case it's present)
/// as well as any additional cleanup as required.
pub fn save_rom(emulator: &Emulator) {
    if let Some(ram) = emulator.battery_ram() {
        let save_dir = ProjectDirs::from("", "Hirtol",  "Rustyboi")
            .expect("Could not get access to data dir for saving!").data_dir().join("saves");
        create_dir_all(&save_dir);
        // Really, this expect case shouldn't ever be reached.
        let title = emulator.game_title().expect("No cartridge loaded, can't save!");

        let mut save_file = File::create(save_dir.join(format!("{}.save", title)))
            .expect("Could not create the save file");
        save_file.write(ram);

        log::debug!("Finished saving the external ram with size: {} successfully!", ram.len());
    }
}

/// Create an emulator for the ROM provided by `rom_path`.
/// In case the file provided is not a rom the program will *probably* crash.
///
/// Any external ram will also automatically be loaded if present.
pub fn create_emulator(rom_path: impl AsRef<Path>, boot_rom: Option<[u8; 256]>) -> Emulator {
    let rom = read(rom_path.as_ref()).expect(&format!("Could not open ROM file {:?}!", rom_path.as_ref()));
    let saved_ram = find_saved_ram(find_rom_name(&rom));

    log::info!("Created emulator for Path {:?} with saved data: {}", rom_path.as_ref(), saved_ram.is_some());

    Emulator::new(boot_rom, &rom, saved_ram)
}

pub fn find_saved_ram<'a>(name: impl AsRef<str>) -> Option<Vec<u8>> {
    let save_dir = ProjectDirs::from("", "Hirtol",  "Rustyboi")
        .expect("Could not get access to data dir for saving!").data_dir().join("saves");
    create_dir_all(&save_dir);

    read(save_dir.join(format!("{}.save", name.as_ref()))).ok()
}

pub fn find_rom_name(rom: &[u8]) -> String {
    Cartridge::new(rom, None).cartridge_header().title.clone()
}

pub fn vec_to_bootrom(vec: &Vec<u8>) -> [u8; 256] {
    let mut result = [0u8; 256];

    for (i, instr) in vec.iter().enumerate() {
        result[i] = *instr;
    }

    result
}