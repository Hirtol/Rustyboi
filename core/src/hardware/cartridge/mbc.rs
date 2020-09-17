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
    ram: [u8; EXTERNAL_RAM_SIZE],
}

impl MBC0 {
    pub fn new(rom: Vec<u8>) -> Self {
        MBC0 {
            rom,
            ram: [0xFF; EXTERNAL_RAM_SIZE],
        }
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
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => {
                self.ram[(address - EXTERNAL_RAM_START) as usize] = value
            }
            _ => return,
        }
    }
}

pub struct MBC1 {
    has_battery: bool,
    ram_enabled: bool,
    banking_mode_select: bool,
    rom_bank: u8,
    ram_bank: u8,
    bank1: u8,
    bank2: u8,
    rom: Vec<u8>,
    effective_banks: u8,
    ram: [u8; EXTERNAL_RAM_SIZE * 4],
}

impl MBC1 {
    pub fn new(rom: Vec<u8>, has_battery: bool) -> Self {
        log::info!("Size: {} - Result calc: {}", rom.len(), (rom.len() % (EXTERNAL_RAM_SIZE * 2)));
        MBC1 {
            ram_enabled: false,
            has_battery,
            banking_mode_select: false,
            rom_bank: 1,
            ram_bank: 0,
            bank1: 0,
            effective_banks: (rom.len() / (EXTERNAL_RAM_SIZE * 2)) as u8,
            rom,
            ram: [INVALID_READ; EXTERNAL_RAM_SIZE * 4],
            bank2: 0
        }
    }

    #[inline]
    fn enable_ram(&mut self, value: u8) {
        self.ram_enabled = (value & 0xF) == 0xA;
    }

    #[inline]
    fn set_lower_rom_bank(&mut self, value: u8) {
        // Mask first 5 bits. May need to base this off actual cartridge size according to docs.
        self.bank1 = value & 0x1F;
        self.rom_bank &= 0xE0;

        if self.bank1 == 0 {
            // Can't ever select ROM bank 0 directly.
            self.bank1 = 0x1;
        }

        self.rom_bank |= self.bank1;
    }

    #[inline]
    fn set_higher_rom_bank(&mut self, value: u8) {
        // Preemptively shift the bank 2 bits 5 bits to the left.
        // Done because every operation after this will have them as such anyway.
        self.bank2 = (value & 0x03) << 5;

        if !self.banking_mode_select {
            // ROM Banking
            self.rom_bank &= 0x1F; // Turn off bits 5 and 6.
            self.rom_bank |= self.bank2; // Set bits 5 and 6.
        }
    }

    fn write_ram(&mut self, address: u16, value: u8) {
        if self.ram_enabled {
            let true_address = (address - EXTERNAL_RAM_START) as usize;
            if !self.banking_mode_select {
                self.ram[true_address] = value;
            } else {
                let offset = self.ram_bank as usize * EXTERNAL_RAM_SIZE;
                self.ram[true_address + offset] = value;
            }
        }
    }
}
//TODO: Fix this implementation with proper masks etc.
impl MBC for MBC1 {
    fn read_3fff(&self, address: u16) -> u8 {
        if !self.banking_mode_select {
            self.rom[address as usize]
        } else {
            let mut effective_address = (address as usize) & 0x3FFF;
            //effective_address = effective_address | ((self.bank2 as usize) << 14);
            self.rom[effective_address]
        }
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


#[cfg(test)]
mod tests {
    use crate::hardware::cartridge::mbc::{MBC1, EXTERNAL_RAM_SIZE};
    use crate::hardware::cartridge::MBC;

    #[test]
    fn basic_mbc_1_test() {
        // 64KB rom, 4 banks
        let mut rom = vec![0x0_u8; EXTERNAL_RAM_SIZE*8];
        // Set the upper 2 banks to something different.
        set_rom_bank_to_value(&mut rom, 1, 0x1);
        set_rom_bank_to_value(&mut rom, 2, 0x2);
        set_rom_bank_to_value(&mut rom, 3, 0x3);

        let mut mbc = MBC1::new(rom, false);

        assert_eq!(mbc.read_3fff(0x500), 0x0);
        assert_eq!(mbc.read_7fff(0x4500), 0x1);

        mbc.write_byte(0x2000, 0b101_0_0010);

        assert_eq!(mbc.read_7fff(0x4500), 0x2);


    }


    fn set_rom_bank_to_value(rom: &mut [u8], rom_bank: usize, value: u8) {
        for i in (EXTERNAL_RAM_SIZE*2*(rom_bank))..(EXTERNAL_RAM_SIZE*2*(rom_bank)+EXTERNAL_RAM_SIZE*2) {
            rom[i] = value;
        }
    }

}
