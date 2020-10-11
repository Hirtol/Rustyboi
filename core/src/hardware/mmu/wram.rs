use crate::hardware::mmu::{INVALID_READ, WRAM_BANK_00_START};

pub const WRAM_BANK_SIZE: usize = 0x1000;
pub const WRAM_SIZE: usize = WRAM_BANK_SIZE * 8;

const WRAM_OFFSET: u16 = WRAM_BANK_00_START;
const ECHO_RAM_OFFSET: u16 = crate::hardware::mmu::ECHO_RAM_START;

/// Work ram is 8KB in DMG mode, and 32 KB in CGB mode, we'll just allocate 32KB regardless
#[derive(Debug)]
pub struct Wram {
    memory: [u8; WRAM_SIZE]
}

impl Wram {
    pub fn new() -> Self {
        Wram { memory: [INVALID_READ; WRAM_SIZE] }
    }

    pub fn read_bank_0(&self, address: u16) -> u8 {
        self.memory[(address - WRAM_OFFSET) as usize]
    }

    pub fn read_bank_n(&self, address: u16) -> u8 {
        self.memory[(address - WRAM_OFFSET) as usize]
    }

    pub fn read_echo_ram(&self, address: u16) -> u8 {
        self.memory[(address - ECHO_RAM_OFFSET) as usize]
    }

    pub fn write_bank_0(&mut self, address: u16, value: u8) {
        self.memory[(address - WRAM_OFFSET) as usize] = value;
    }

    pub fn write_bank_n(&mut self, address: u16, value: u8) {
        self.memory[(address - WRAM_OFFSET) as usize] = value;
    }

    pub fn write_echo_ram(&mut self, address: u16, value: u8) {
        self.memory[(address - ECHO_RAM_OFFSET) as usize] = value;
    }
}