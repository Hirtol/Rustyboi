use crate::io::bootrom::BootRom;
use crate::hardware::cartridge::Cartridge;
use bitflags::_core::fmt::{Debug, Formatter};
use std::fmt;

pub const MEMORY_SIZE: usize = 0x10000;
// 16 KB ROM bank, usually 00. From Cartridge, read-only
pub const ROM_BANK_00_START: u16 = 0x0000;
pub const ROM_BANK_00_END: u16 = 0x03FFF;
// 16 KB Rom Bank 01~NN. From cartridge, switchable bank via Memory Bank. Read-only.
pub const ROM_BANK_NN_START: u16 = 0x4000;
pub const ROM_BANK_NN_END: u16 = 0x7FFF;
//This area contains information about the program,
// its entry point, checksums, information about the used MBC chip, the ROM and RAM sizes, etc.
pub const CARTRIDGE_HEADER_START: u16 = 0x0100;
pub const CARTRIDGE_HEADER_END: u16 = 0x014F;
// 8 KB of VRAM, only bank 0 in Non-CGB mode. Switchable bank 0/1 in CGB mode.
pub const VRAM_START: u16 = 0x8000;
pub const VRAM_END: u16 = 0x9FFF;
// 8 KB of External Ram, In cartridge, switchable bank if any(?). Could hold save data.
pub const EXTERNAL_RAM_START: u16 = 0xA000;
pub const EXTERNAL_RAM_END: u16 = 0xBFFF;
// 4 KB Work RAM bank 0
pub const WRAM_BANK_00_START: u16 = 0xC000;
pub const WRAM_BANK_00_END: u16 = 0xCFFF;
// 4 KB Work RAM bank 1~N. Only bank 1 in Non-CGB mode Switchable bank 1~7 in CGB mode.
pub const WRAM_BANK_NN_START: u16 = 0xD000;
pub const WRAM_BANK_NN_END: u16 = 0xDFFF;
//Mirror of C000~DDFF (ECHO RAM). Typically not used
pub const ECHO_RAM_START: u16 = 0xE000;
pub const ECHO_RAM_END: u16 = 0xFDFF;
// Sprite attribute table (OAM)
pub const SPRITE_ATTRIBUTE_START: u16 = 0xFE00;
pub const SPRITE_ATTRIBUTE_END: u16 = 0xFE9F;
// Not usable
pub const NOT_USABLE_START: u16 = 0xFEA0;
pub const NOT_USABLE_END: u16 = 0xFEFF;
// I/O Registers
pub const IO_START: u16 = 0xFF00;
pub const IO_END: u16 = 0xFF7F;
// High Ram (HRAM)
pub const HRAM_START: u16 = 0xFF80;
pub const HRAM_END: u16 = 0xFFFE;
// Interrupts Enable Register (IE)
pub const INTERRUPTS_REGISTER_START: u16 = 0xFFFF;
pub const INTERRUPTS_REGISTER_END: u16 = 0xFFFF;

pub struct Memory {
    memory: Vec<u8>,
    pub boot_rom: BootRom,
    cartridge: Cartridge,
}

impl Memory {
    pub fn new(boot_rom: Option<[u8; 0x100]>, cartridge: &[u8]) -> Self {
        Memory {
            memory: vec![0u8; MEMORY_SIZE],
            boot_rom: BootRom::new(boot_rom),
            cartridge: Cartridge::new(cartridge),
        }
    }

    pub fn read_byte(&self, address: u16) -> u8
    {
        if cfg!(test) {
            return self.memory[address as usize];
        }

        match address >> 8 {
            // Due to strong coupeling between all components I've kinda screwed myself
            // over with my CPU tests, until we decouple our architecture in a refactoring this'll
            // have to stay
            0x0000..=0x00FF if !self.boot_rom.is_finished => self.boot_rom.read_byte(address),
            ROM_BANK_00_START..=ROM_BANK_00_END => self.cartridge.read_0000_3fff(address),
            ROM_BANK_NN_START..=ROM_BANK_NN_END => self.cartridge.read_4000_7fff(address),

            _ => self.memory[address as usize]
        }
    }

    pub fn set_byte(&mut self, address: u16, value: u8) {
        //TODO: Add bound checks to ensure we're not accessing protected memory.
        self.memory[address as usize] = value;
    }

    pub fn read_short(&self, address: u16) -> u16 {
        let least_s_byte = self.read_byte(address) as u16;
        let most_s_byte = self.read_byte(address.wrapping_add(1)) as u16;

        (most_s_byte << 8) | least_s_byte
    }

    pub fn set_short(&mut self, address: u16, value: u16) {
        self.set_byte(address, (value & 0xFF) as u8); // Least significant byte first.
        self.set_byte(address.wrapping_add(1), ((value & 0xFF00) >> 8) as u8);
    }
}

impl Debug for Memory{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Memory: {:?}\nCartridge: {:?}", self.memory, self.cartridge)
    }
}
