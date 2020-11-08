use crate::hardware::mmu::{HRAM_START, INVALID_READ};

/// FFFE - FF80 = 0x7E+1 for array size
pub const HRAM_SIZE: usize = 0x7F;
const HRAM_OFFSET: u16 = HRAM_START;

#[derive(Debug)]
pub struct Hram {
    memory: [u8; HRAM_SIZE],
}

impl Hram {
    pub fn new() -> Self {
        Hram {
            memory: [INVALID_READ; HRAM_SIZE],
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.memory[(address - HRAM_OFFSET) as usize]
    }

    pub fn set_byte(&mut self, address: u16, value: u8) {
        self.memory[(address - HRAM_OFFSET) as usize] = value;
    }
}
