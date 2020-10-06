use crate::hardware::cartridge::header::RamSizes;
use crate::hardware::cartridge::mbc::{EXTERNAL_RAM_SIZE, MBC, ROM_BANK_SIZE};
use crate::hardware::mmu::{EXTERNAL_RAM_END, EXTERNAL_RAM_START, INVALID_READ};

//TODO: Create MBC3 + Timer
pub struct MBC3 {
    has_battery: bool,
    ram_enabled: bool,
    rom_bank: u16,
    ram_bank: u8,
    effective_banks: u16,
    rom: Vec<u8>,
    ram: Vec<u8>,
}

impl MBC3 {
    pub fn new(rom: Vec<u8>, has_battery: bool, ram_size: &RamSizes, saved_ram: Option<Vec<u8>>) -> Self {
        log::info!(
            "MBC3 ROM Size: {} - Effective banks: {}",
            rom.len(),
            (rom.len() / ROM_BANK_SIZE)
        );
        let mut result = MBC3 {
            ram_enabled: false,
            has_battery,
            rom_bank: 1,
            ram_bank: 0,
            effective_banks: (rom.len() / ROM_BANK_SIZE) as u16,
            rom,
            ram: vec![INVALID_READ; ram_size.to_usize()],
        };

        if let Some(ram) = saved_ram {
            result.ram = ram;
        }

        result
    }

    #[inline]
    fn set_lower_rom_bank(&mut self, value: u8) {
        // Select the first 7 bits and use that as the bank number.
        self.rom_bank = (value & 0x7F) as u16;

        if self.rom_bank == 0 {
            self.rom_bank = 1;
        }

        self.rom_bank %= self.effective_banks;
    }
}

impl MBC for MBC3 {
    fn read_3fff(&self, address: u16) -> u8 {
        // MBC5 will always have the first 16KB of the rom mapped to the lower range \o/
        self.rom[address as usize]
    }

    fn read_7fff(&self, address: u16) -> u8 {
        // first 14 bits of the address, and then the rom bank shifted onto it.
        let result_address = (address & 0x3FFF) as usize | (self.rom_bank as usize) << 14;
        self.rom[result_address]
    }

    fn read_ex_ram(&self, address: u16) -> u8 {
        if self.ram_enabled {
            let true_address = (address - EXTERNAL_RAM_START) as usize + EXTERNAL_RAM_SIZE * self.ram_bank as usize;
            self.ram[true_address]
        } else {
            INVALID_READ
        }
    }

    fn get_battery_ram(&self) -> Option<&[u8]> {
        if self.has_battery {
            Some(&self.ram)
        } else {
            None
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => self.ram_enabled = (value & 0xF) == 0xA,
            0x2000..=0x3FFF => self.set_lower_rom_bank(value),
            //TODO: Ram bank?
            0x4000..=0x5FFF => self.ram_bank = value & 0xA,
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => {
                if self.ram_enabled {
                    let true_address = (address - EXTERNAL_RAM_START) as usize;
                    let offset = EXTERNAL_RAM_SIZE * self.ram_bank as usize;
                    self.ram[offset + true_address] = value;
                }
            }
            _ => return,
        }
    }
}
