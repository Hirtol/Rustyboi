use crate::hardware::mmu::INVALID_READ;

pub const IO_SIZE: usize = 0x80;

/// A struct for the miscellaneous I/O registers
#[derive(Debug)]
pub struct IORegisters {
    memory: [u8; IO_SIZE],
}

impl IORegisters {
    pub fn new() -> Self {
        IORegisters {
            memory: [INVALID_READ; IO_SIZE],
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.memory[(address & 0xFF) as usize]
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        self.memory[(address & 0xFF) as usize] = value;
    }
}
