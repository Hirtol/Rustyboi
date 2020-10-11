use crate::hardware::mmu::{IO_START, INVALID_READ};

pub const IO_SIZE: usize = 0x80;

pub const SIO_DATA: u16 = 0xFF01;
/// FF02 -- SIOCONT [RW] Serial I/O Control       | when set to 1 | when set to 0
/// Bit7  Transfer start flag                     | START         | NO TRANSFER
/// Bit0  Serial I/O clock select                 | INTERNAL      | EXTERNAL
pub const SIO_CONT: u16 = 0xFF02;

/// A struct for the miscellaneous I/O registers
#[derive(Debug)]
pub struct IORegisters {
    memory: [u8; IO_SIZE],
}

impl IORegisters {
    pub fn new() -> Self {
        IORegisters{ memory: [INVALID_READ; IO_SIZE] }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.memory[(address & 0xFF) as usize]
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        self.memory[(address & 0xFF) as usize] = value;
    }
}