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

use crate::hardware::cartridge::header::RamSizes;
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
    bank1: u8,
    bank2: u8,
    effective_banks: u8,
    rom: Vec<u8>,
    ram: Vec<u8>,
}

impl MBC1 {
    pub fn new(rom: Vec<u8>, has_battery: bool, ram_size: &RamSizes) -> Self {
        log::info!("Size: {} - Effective banks: {}", rom.len(), (rom.len() / (EXTERNAL_RAM_SIZE * 2)));
        MBC1 {
            ram_enabled: false,
            has_battery,
            banking_mode_select: false,
            rom_bank: 1,
            bank1: 1,
            effective_banks: (rom.len() / (EXTERNAL_RAM_SIZE * 2)) as u8,
            rom,
            ram: vec![INVALID_READ; ram_size.to_usize()],
            bank2: 0,
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

        if self.bank1 == 0 {
            // Can't ever select ROM bank 0 directly.
            self.bank1 = 0x1;
        }

        self.rom_bank = self.bank2 | self.bank1;
        self.rom_bank %= self.effective_banks;
    }

    #[inline]
    fn set_higher_rom_bank(&mut self, value: u8) {
        // Preemptively shift the bank 2 bits 5 bits to the left.
        // Done because every operation after this will have them as such anyway.
        self.bank2 = (value & 0x03) << 5;
        self.rom_bank = self.bank2 | self.bank1;
        self.rom_bank %= self.effective_banks;
    }

    fn write_ram(&mut self, address: u16, value: u8) {
        if self.ram_enabled {
            let result_address = (address & 0x1FFF) as usize;
            if !self.banking_mode_select {
                self.ram[result_address] = value
            } else {
                // If we only have enough bytes for one bank we ignore any bank switching.
                if self.ram.len() > 8192 {
                    self.ram[result_address | ((self.bank2 as usize) << 8)] = value;
                } else {
                    self.ram[result_address] = value
                }
            }
        }
    }
}

impl MBC for MBC1 {
    fn read_3fff(&self, address: u16) -> u8 {
        if !self.banking_mode_select {
            self.rom[address as usize]
        } else {
            // first 14 bits of the address, and then the rom bank shifted onto the upper 7 bits.
            // This results in a total address space of 21 bits.
            let result_address = (address & 0x3FFF) as usize | ((self.bank2 % self.effective_banks) as usize) << 14;

            self.rom[result_address]
        }
    }

    fn read_7fff(&self, address: u16) -> u8 {
        // first 14 bits of the address, and then the rom bank shifted onto it.
        let result_address = (address & 0x3FFF) as usize | (self.rom_bank as usize) << 14;
        self.rom[result_address]
    }

    fn read_ex_ram(&self, address: u16) -> u8 {
        if self.ram_enabled {
            // Only the lower 13 bits are relevant for RAM addressing.
            let result_address = (address & 0x1FFF) as usize;
            if !self.banking_mode_select {
                self.ram[result_address]
            } else {
                // If we only have 8KB we don't need banking.
                if self.ram.len() > 8192 {
                    self.ram[result_address | ((self.bank2 as usize) << 8)]
                }else {
                    self.ram[result_address]
                }
            }
        } else {
            INVALID_READ
        }
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
    use crate::hardware::cartridge::header::RamSizes::KB32;
    use crate::hardware::cartridge::mbc::{EXTERNAL_RAM_SIZE, MBC1};
    use crate::hardware::cartridge::MBC;

    #[test]
    fn basic_mbc_1_test() {
        // 64 KB, 4 banks
        let mut mbc = get_basic_mbc1(EXTERNAL_RAM_SIZE * 8);

        assert_eq!(mbc.read_3fff(0x500), 0x0);
        assert_eq!(mbc.read_7fff(0x4500), 0x1);

        mbc.write_byte(0x2000, 0b101_0_0010);

        assert_eq!(mbc.read_7fff(0x4500), 0x2);
    }

    #[test]
    fn test_effective_rom_bank_mbc1() {
        let mut mbc = get_basic_mbc1(EXTERNAL_RAM_SIZE * 8);
        // Bank 1 write
        mbc.write_byte(0x2000, 0b101_1_0010);
        // Bank 2 write
        mbc.write_byte(0x4500, 0b1111_0001);

        // Ensure we wrap around properly.
        assert_eq!(mbc.rom_bank, 0b10);
        assert_eq!(mbc.read_7fff(0x4500), 2);
    }

    #[test]
    fn test_bank_mode_mbc1() {
        // 64 banks
        let mut mbc = get_basic_mbc1(EXTERNAL_RAM_SIZE * 128);
        // Bank 1 write
        mbc.write_byte(0x2000, 0b101_1_0010);
        // Bank 2 write
        mbc.write_byte(0x4500, 0b1111_0001);
        // Turn on bank mode
        mbc.write_byte(0x6000, 0x1);

        assert_eq!(mbc.read_3fff(0x2000), 32);
    }

    #[test]
    fn test_mooneye_example_mbc1() {
        // 256 banks
        let mut mbc = get_basic_mbc1(EXTERNAL_RAM_SIZE * 256);
        // Bank 1 write
        mbc.write_byte(0x2000, 0b101_0_0100);
        // Bank 2 write
        mbc.write_byte(0x4500, 0b1111_0010);
        // Should be the 68th rom bank.
        assert_eq!(mbc.rom_bank, 0b1000100);
        // Ensure we're reading from the 68th rom bank
        assert_eq!(mbc.read_7fff(0x72A7), 68);
    }

    #[test]
    fn test_mooneye_ram_example_mbc1() {
        // 256 banks
        let mut mbc = get_basic_mbc1(EXTERNAL_RAM_SIZE * 256);

        set_ram_bank_to_value(&mut mbc.ram, 0, 0);
        set_ram_bank_to_value(&mut mbc.ram, 1, 1);
        set_ram_bank_to_value(&mut mbc.ram, 2, 2);
        // Enable ram
        mbc.write_byte(0x1000, 0xA);
        // Bank 2 write
        mbc.write_byte(0x4500, 0b1111_0010);
        // Ensure we're reading from the 0th ram bank
        assert_eq!(mbc.read_ex_ram(0xA300), 0);
        assert_eq!(mbc.read_ex_ram(0xB123), 0);
        // Turn on bank mode
        mbc.write_byte(0x6000, 0x1);

        //println!("{:?}", mbc.ram.to_vec());

        assert_eq!(mbc.read_ex_ram(0xB123), 2);
    }

    fn get_basic_mbc1(size: usize) -> MBC1 {
        let mut rom = vec![0x0_u8; size];
        // Set the upper 2 banks to something different.
        for i in 0..(size / (EXTERNAL_RAM_SIZE * 2)) {
            set_rom_bank_to_value(&mut rom, i, i as u8);
        }

        MBC1::new(rom, false, &KB32)
    }

    fn set_rom_bank_to_value(rom: &mut [u8], rom_bank: usize, value: u8) {
        for i in (EXTERNAL_RAM_SIZE * 2 * (rom_bank))..(EXTERNAL_RAM_SIZE * 2 * (rom_bank) + EXTERNAL_RAM_SIZE * 2) {
            rom[i] = value;
        }
    }

    fn set_ram_bank_to_value(rom: &mut [u8], rom_bank: usize, value: u8) {
        for i in (EXTERNAL_RAM_SIZE * (rom_bank))..(EXTERNAL_RAM_SIZE * (rom_bank) + EXTERNAL_RAM_SIZE) {
            rom[i] = value;
        }
    }
}
