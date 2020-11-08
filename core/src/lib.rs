pub mod emulator;
mod scheduler;
use crate::emulator::EmulatorMode;
use crate::hardware::ppu::palette::DisplayColour;
pub use crate::io::joypad::InputKey;

pub mod hardware;
mod io;

/// Struct for wrapping all the various options for the `Emulator`
#[derive(Debug)]
pub struct EmulatorOptions {
    pub boot_rom: Option<Vec<u8>>,
    pub saved_ram: Option<Vec<u8>>,
    pub emulator_mode: EmulatorMode,
    pub display_colour: DisplayColour,
}

#[derive(Debug)]
pub struct EmulatorOptionsBuilder {
    boot_rom: Option<Vec<u8>>,
    saved_ram: Option<Vec<u8>>,
    emulator_mode: EmulatorMode,
    display_colour: DisplayColour,
}

impl EmulatorOptionsBuilder {
    pub fn new() -> Self {
        EmulatorOptionsBuilder {
            boot_rom: None,
            saved_ram: None,
            emulator_mode: EmulatorMode::DMG,
            display_colour: DisplayColour::default(),
        }
    }

    pub fn boot_rom(mut self, boot_rom: Option<Vec<u8>>) -> Self {
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

    pub fn with_display_colours(mut self, colours: DisplayColour) -> Self {
        self.display_colour = colours;
        self
    }

    pub fn build(self) -> EmulatorOptions {
        EmulatorOptions {
            boot_rom: self.boot_rom,
            saved_ram: self.saved_ram,
            emulator_mode: self.emulator_mode,
            display_colour: self.display_colour,
        }
    }
}

impl From<EmulatorOptions> for EmulatorOptionsBuilder {
    fn from(from: EmulatorOptions) -> Self {
        EmulatorOptionsBuilder {
            boot_rom: from.boot_rom,
            saved_ram: from.saved_ram,
            emulator_mode: from.emulator_mode,
            display_colour: from.display_colour,
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
