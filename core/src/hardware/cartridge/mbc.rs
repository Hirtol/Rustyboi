// enum MBC {
//     NONE,
//     MBC1 {ram_enabled: bool, },
//     MBC2,
//     MBC3,
//     MBC5,
//
//     MBC6,
//     MBC7,
//     HuC1,
// }

use crate::hardware::cartridge::MBC;
use crate::hardware::memory::*;

// 8 KB
const EXTERNAL_RAM_SIZE: usize = 8192;

/// Struct representing No MBC
pub struct MBC0 {
    rom: Vec<u8>,
    ram: [u8; EXTERNAL_RAM_SIZE]
}

impl MBC0 {
    pub fn new(rom: Vec<u8>) -> Self {
        MBC0 { rom, ram: [0xFF; EXTERNAL_RAM_SIZE] }
    }
}

impl MBC for MBC0 {
    fn read_3fff(&self, address: u16) -> u8 {
        self.rom[address as usize]
    }

    fn read_7fff(&self, address: u16) -> u8 {
        self.rom[address as usize]
    }

    fn read_ex_ram(&self, address: u16) -> u8 {
        self.ram[(address - EXTERNAL_RAM_START) as usize]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => self.ram[(address - EXTERNAL_RAM_START) as usize] = value,
            _ => return,
        }
    }
}

pub struct MBC1 {
    ram_enabled: bool,
    banking_mode_select: bool,
    rom_bank: u8,
    ram_bank: u8,
    rom: Vec<u8>,
    ram: [u8; EXTERNAL_RAM_SIZE * 4],
}

impl MBC1 {
    pub fn new(rom: Vec<u8>) -> Self {
        MBC1 { ram_enabled: false, banking_mode_select: false, rom_bank: 1, ram_bank: 0, rom, ram: [0xFF; EXTERNAL_RAM_SIZE*4] }
    }

    #[inline]
    fn enable_ram(&mut self, value: u8) {
        self.ram_enabled = (value & 0xF) == 0xA;
    }

    #[inline]
    fn set_lower_rom_bank(&mut self, value: u8) {
        // Mask first 5 bits. May need to base this off actual cartridge size according to docs.
        let rom_bank = value & 0x1F;
        self.rom_bank &= 0xE0;
        self.rom_bank |= rom_bank;
        if self.rom_bank == 0 {
            // Can't ever select ROM bank 0 directly.
            self.rom_bank = 1;
        }
    }

    #[inline]
    fn set_higher_rom_bank(&mut self, value: u8) {
        if !self.banking_mode_select{
            // ROM Banking
            let rom_bank = (value<<5); // Move bits into correct location.
            self.rom_bank &= 0x60; // Turn off bits 5 and 6.
            self.rom_bank |= rom_bank; // Set bits 5 and 6.
        } else {
            // RAM Banking
            self.ram_bank = value&0x03;
        }
    }

    fn write_ram(&mut self, address: u16, value: u8) {
        if self.ram_enabled {
            let true_address = (address - EXTERNAL_RAM_START) as usize;
            if !self.banking_mode_select {
                self.ram[true_address] = value;
            }else {
                let offset = self.ram_bank as usize * EXTERNAL_RAM_SIZE;
                self.ram[true_address + offset] = value;
            }
        }
    }
}
//TODO: Fix this implementation with proper masks etc.
impl MBC for MBC1 {
    fn read_3fff(&self, address: u16) -> u8 {
        self.rom[address as usize]
    }

    fn read_7fff(&self, address: u16) -> u8 {
        let offset = ROM_BANK_NN_START * self.rom_bank as u16;
        self.rom[((address - ROM_BANK_NN_START) + offset) as usize]
    }

    fn read_ex_ram(&self, address: u16) -> u8 {
        // Currently we do no check that the cartridge actually has ram..
        let offset = EXTERNAL_RAM_SIZE * self.ram_bank as usize;
        self.ram[(address - EXTERNAL_RAM_START) as usize + offset]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => self.enable_ram(value),
            0x2000..=0x3FFF => self.set_lower_rom_bank(value),
            0x4000..=0x5FFF => self.set_higher_rom_bank(value),
            0x6000..=0x7FFF => self.banking_mode_select = value == 1,
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => self.write_ram(address, value),
            _ => return,
        }
    }
}