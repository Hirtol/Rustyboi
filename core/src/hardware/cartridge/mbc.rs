use crate::hardware::cartridge::header::RamSizes;
use crate::hardware::mmu::*;

pub const EXTERNAL_RAM_SIZE: usize = 8192;
pub const ROM_BANK_SIZE: usize = 16384;

#[derive(Debug)]
pub enum MBC {
    MBC0,
    MBC1(MBC1State),
    MBC3(MBC3State),
    MBC5(MBC5State),
}

#[derive(Debug, Clone)]
pub struct MBC1State {
    pub ram_enabled: bool,
    banking_mode_select: bool,
    rom_bank: u8,
    bank1: u8,
    bank2: u8,
}

impl Default for MBC1State {
    fn default() -> Self {
        MBC1State {
            ram_enabled: false,
            banking_mode_select: false,
            rom_bank: 1,
            bank1: 1,
            bank2: 0,
        }
    }
}

impl MBC1State {
    pub fn get_3fff_offset(&self, effective_rom_banks: usize) -> usize {
        if self.banking_mode_select {
            // first 14 bits of the address, and then the rom bank shifted onto the upper 7 bits.
            // This results in a total address space of 21 bits.
            (self.bank2 as usize % effective_rom_banks) << 14
        } else {
            0
        }
    }

    pub fn get_7fff_offset(&self) -> usize {
        (self.rom_bank as usize) << 14
    }

    pub fn get_ram_offset(&self, ram_length: usize) -> usize {
        if self.banking_mode_select && ram_length > 8192 {
            ((self.bank2 as usize) << 8)
        } else {
            0
        }
    }

    pub fn enable_ram(&mut self, value: u8) {
        self.ram_enabled = (value & 0xF) == 0xA;
    }

    pub fn set_lower_rom_bank(&mut self, value: u8, effective_rom_banks: usize) {
        // Mask first 5 bits. May need to base this off actual cartridge size according to docs.
        self.bank1 = value & 0x1F;

        if self.bank1 == 0 {
            // Can't ever select ROM bank 0 directly.
            self.bank1 = 0x1;
        }

        self.rom_bank = self.bank2 | self.bank1;
        self.rom_bank %= effective_rom_banks as u8;
    }

    pub fn set_higher_rom_bank(&mut self, value: u8, effective_rom_banks: usize) {
        // Preemptively shift the bank 2 bits 5 bits to the left.
        // Done because every operation after this will have them as such anyway.
        self.bank2 = (value & 0x03) << 5;
        self.rom_bank = self.bank2 | self.bank1;
        self.rom_bank %= effective_rom_banks as u8;
    }

    pub fn set_bank_mode_select(&mut self, value: u8) {
        self.banking_mode_select = value == 1
    }
}

#[derive(Debug, Clone)]
pub struct MBC3State {
    pub ram_enabled: bool,
    rom_bank: u16,
    ram_bank: u8,
}

impl Default for MBC3State {
    fn default() -> Self {
        MBC3State {
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
        }
    }
}

impl MBC3State {
    pub fn get_3fff_offset(&self) -> usize {
        0
    }

    pub fn get_7fff_offset(&self) -> usize {
        (self.rom_bank as usize) << 14
    }

    pub fn get_ram_offset(&self) -> usize {
        EXTERNAL_RAM_SIZE * self.ram_bank as usize
    }

    pub fn enable_ram(&mut self, value: u8) {
        self.ram_enabled = (value & 0xF) == 0xA;
    }

    pub fn write_lower_rom_bank(&mut self, value: u8, effective_rom_banks: usize) {
        // Select the first 7 bits and use that as the bank number.
        self.rom_bank = (value & 0x7F) as u16;

        if self.rom_bank == 0 {
            self.rom_bank = 1;
        }

        self.rom_bank %= effective_rom_banks as u16;
    }

    pub fn write_ram_bank(&mut self, value: u8) {
        self.ram_bank = value & 0xA
    }
}

#[derive(Debug, Clone)]
pub struct MBC5State {
    pub ram_enabled: bool,
    rom_bank: u16,
    ram_bank: u8,
}

impl Default for MBC5State {
    fn default() -> Self {
        MBC5State {
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
        }
    }
}

impl MBC5State {
    pub fn get_3fff_offset(&self) -> usize {
            0
    }

    pub fn get_7fff_offset(&self) -> usize {
        (self.rom_bank as usize) << 14
    }

    pub fn get_ram_offset(&self) -> usize {
        EXTERNAL_RAM_SIZE * self.ram_bank as usize
    }

    pub fn enable_ram(&mut self, value: u8) {
        self.ram_enabled = value == 0b0000_1010;
    }

    pub fn write_lower_rom_bank(&mut self, value: u8, effective_rom_banks: usize) {
        self.rom_bank = (self.rom_bank & 0x100) | value as u16;
        self.rom_bank %= effective_rom_banks as u16;
    }

    pub fn write_higher_rom_bank(&mut self, value: u8, effective_rom_banks: usize) {
        self.rom_bank = ((value as u16) << 8) | (self.rom_bank & 0xFF);
        self.rom_bank %= effective_rom_banks as u16;
    }

    pub fn write_ram_bank(&mut self, value: u8) {
        self.ram_bank = value & 0xF
    }
}
