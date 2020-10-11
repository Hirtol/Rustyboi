pub mod emulator;
mod scheduler;
pub use crate::hardware::ppu::palette::DmgColor;
pub use crate::io::joypad::InputKey;
use crate::emulator::EmulatorMode;

pub mod hardware;
mod io;

/// Struct for wrapping all the various options for the `Emulator`
#[derive(Debug)]
pub struct EmulatorOptions {
    pub boot_rom: Option<[u8; 256]>,
    pub saved_ram: Option<Vec<u8>>,
    pub emulator_mode: EmulatorMode,
}

#[derive(Debug)]
pub struct EmulatorOptionsBuilder {
    boot_rom: Option<[u8; 256]>,
    saved_ram: Option<Vec<u8>>,
    emulator_mode: EmulatorMode,
}

impl EmulatorOptionsBuilder {
    pub fn new() -> Self {
        EmulatorOptionsBuilder{
            boot_rom: None,
            saved_ram: None,
            emulator_mode: EmulatorMode::DMG
        }
    }

    pub fn boot_rom(mut self, boot_rom: Option<[u8; 256]>) -> Self {
        self.boot_rom = boot_rom;
        self
    }

    pub fn saved_ram(mut self, saved_ram: Option<Vec<u8>>) -> Self {
        self.saved_ram = saved_ram;
        self
    }

    pub fn with_mode(mut self, mode: EmulatorMode) -> Self {
        self.emulator_mode = mode;
        self
    }

    pub fn build(self) -> EmulatorOptions {
        EmulatorOptions{
            boot_rom: self.boot_rom,
            saved_ram: self.saved_ram,
            emulator_mode: self.emulator_mode
        }
    }
}

impl From<EmulatorOptions> for EmulatorOptionsBuilder {
    fn from(from: EmulatorOptions) -> Self {
        EmulatorOptionsBuilder {
            boot_rom: from.boot_rom,
            saved_ram: from.saved_ram,
            emulator_mode: from.emulator_mode
        }
    }
}

fn print_array_raw<T: Sized>(array: T) {
    let view = &array as *const _ as *const u8;
    for i in 0..(4 * 40) {
        if i % 16 == 0 {
            println!();
        }
        print!("{:02X} ", unsafe { *view.offset(i) });
    }
    println!();
}
