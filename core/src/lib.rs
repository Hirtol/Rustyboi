pub mod emulator;
mod scheduler;
use crate::emulator::GameBoyModel;
use crate::hardware::mmu::INVALID_READ;
use crate::hardware::ppu::palette::DisplayColour;
pub use crate::io::joypad::InputKey;
use bitflags::_core::ops::Deref;
use std::fmt::Debug;
use std::ops::DerefMut;

mod emulator_debug;
pub mod hardware;
mod io;

pub trait ExternalRamBacking: DerefMut<Target = [u8]> + Debug {
    /// Set the length of the underlying backed memory.
    ///
    /// Called preemptively in the emulator every time we load up the memory to ensure
    /// there are no out of bounds calls
    fn set_length(&mut self, length: usize);
    /// Give the backing memory a chance to persist the memory before the object is destroyed.
    fn save(&mut self);
}

/// Struct for wrapping all the various options for the `Emulator`
#[derive(Debug)]
pub struct EmulatorOptions {
    pub boot_rom: Option<Vec<u8>>,
    pub saved_ram: Option<Vec<u8>>,
    pub emulator_mode: GameBoyModel,
    pub bg_display_colour: DisplayColour,
    pub sp0_display_colour: DisplayColour,
    pub sp1_display_colour: DisplayColour,
}

#[derive(Debug)]
pub struct EmulatorOptionsBuilder {
    boot_rom: Option<Vec<u8>>,
    saved_ram: Option<Vec<u8>>,
    emulator_mode: GameBoyModel,
    bg_display_colour: DisplayColour,
    sp0_display_colour: DisplayColour,
    sp1_display_colour: DisplayColour,
}

impl EmulatorOptionsBuilder {
    pub fn new() -> Self {
        EmulatorOptionsBuilder {
            boot_rom: None,
            saved_ram: None,
            emulator_mode: GameBoyModel::DMG,
            bg_display_colour: Default::default(),
            sp0_display_colour: Default::default(),
            sp1_display_colour: Default::default(),
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

    pub fn with_mode(mut self, mode: GameBoyModel) -> Self {
        self.emulator_mode = mode;
        self
    }

    pub fn with_display_colour(mut self, colours: DisplayColour) -> Self {
        self.bg_display_colour = colours;
        self.sp0_display_colour = colours;
        self.sp1_display_colour = colours;
        self
    }

    pub fn with_bg_display_colour(mut self, colours: DisplayColour) -> Self {
        self.bg_display_colour = colours;
        self
    }

    pub fn with_sp0_display_colour(mut self, colours: DisplayColour) -> Self {
        self.sp0_display_colour = colours;
        self
    }

    pub fn with_sp1_display_colour(mut self, colours: DisplayColour) -> Self {
        self.sp1_display_colour = colours;
        self
    }

    pub fn build(self) -> EmulatorOptions {
        EmulatorOptions {
            boot_rom: self.boot_rom,
            saved_ram: self.saved_ram,
            emulator_mode: self.emulator_mode,
            bg_display_colour: self.bg_display_colour,
            sp0_display_colour: self.sp0_display_colour,
            sp1_display_colour: self.sp1_display_colour,
        }
    }
}

impl From<EmulatorOptions> for EmulatorOptionsBuilder {
    fn from(from: EmulatorOptions) -> Self {
        EmulatorOptionsBuilder {
            boot_rom: from.boot_rom,
            saved_ram: from.saved_ram,
            emulator_mode: from.emulator_mode,
            bg_display_colour: from.bg_display_colour,
            sp0_display_colour: from.sp0_display_colour,
            sp1_display_colour: from.sp1_display_colour,
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
