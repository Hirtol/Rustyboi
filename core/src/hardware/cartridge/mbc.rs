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
    pub ram_bank: u8,
    rom_bank: u16,
    rtc_registers: RTCRegisters,
}

impl Default for MBC3State {
    fn default() -> Self {
        MBC3State {
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
            rtc_registers: RTCRegisters::default(),
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

    pub fn read_rtc_register(&self) -> u8 {
        self.rtc_registers.read_rtc(self.ram_bank)
    }

    pub fn write_rtc_register(&mut self, value: u8) {
        self.rtc_registers.write_rtc(self.ram_bank, value)
    }

    pub fn write_ram_bank(&mut self, value: u8) {
        self.ram_bank = value & 0xF;
    }

    pub fn write_latch_data(&mut self, value: u8) {
        if self.ram_enabled {
            self.rtc_registers.latch_rtc(value);
        }
    }
}

//TODO: Check if we should use user system time to populate these values?
#[derive(Debug, Default, Copy, Clone)]
struct RTCRegisters {
    seconds: u8,
    minutes: u8,
    hours: u8,
    day_counter_lower: u8,
    day_counter_upper: u8,
    latched: bool,
}

impl RTCRegisters {
    #[inline]
    fn latch_rtc(&mut self, value: u8) {
        if !self.latched && value != 0 {
            //TODO: Implement actual timekeeping, look at this:
            // https://web.archive.org/web/20150110235712/https://github.com/supergameherm/supergameherm/blob/df158781fcb85693b3d10fe2f40ea0010573fa5e/src/mbc.c#L378-430
            // for reference.
        }

        self.latched = value == 0;
    }

    #[inline]
    fn read_rtc(&self, address: u8) -> u8 {
        let address = address & 0xF;
        match address {
            0x8 => self.seconds,
            0x9 => self.minutes,
            0xA => self.hours,
            0xB => self.day_counter_lower,
            0xC => self.day_counter_upper,
            _ => unreachable!(),
        }
    }

    #[inline]
    fn write_rtc(&mut self, address: u8, value: u8) {
        let address = address & 0xF;
        match address {
            0x8 => self.seconds = value,
            0x9 => self.minutes = value,
            0xA => self.hours = value,
            0xB => self.day_counter_lower = value,
            0xC => self.day_counter_upper = value,
            _ => unreachable!(),
        }
    }

    fn days(&self) -> u16 {
        ((self.day_counter_upper as u16 & 0x1) << 8) | self.day_counter_lower as u16
    }

    fn clock_halt(&self) -> bool {
        (self.day_counter_upper & 0b0100_0000) != 0
    }

    fn day_overflow(&self) -> bool {
        (self.day_counter_upper & 0b1000_0000) != 0
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
