use crate::hardware::mmu::{INVALID_READ, WRAM_BANK_00_START, WRAM_BANK_NN_START, WRAM_BANK_00_END, WRAM_BANK_NN_END, ECHO_RAM_OFFSET};

pub const WRAM_BANK_SIZE: usize = 0x1000;
pub const WRAM_SIZE: usize = WRAM_BANK_SIZE * 8;

const WRAM_OFFSET: u16 = WRAM_BANK_00_START;
const WRAM_BANK_N_OFFSET: u16 = WRAM_BANK_NN_START;

/// Work ram is 8KB in DMG mode, and 32 KB in CGB mode, we'll just allocate 32KB regardless
#[derive(Debug)]
pub struct Wram {
    memory: [u8; WRAM_SIZE],
    bank_selected: usize,
}

impl Wram {
    pub fn new() -> Self {
        Wram { memory: [INVALID_READ; WRAM_SIZE], bank_selected: 1 }
    }

    pub fn read_bank_0(&self, address: u16) -> u8 {
        self.memory[(address - WRAM_OFFSET) as usize]
    }

    pub fn read_bank_n(&self, address: u16) -> u8 {
        let address = self.bank_selected * WRAM_BANK_SIZE + (address - WRAM_BANK_N_OFFSET) as usize;
        self.memory[address]
    }

    pub fn read_echo_ram(&self, address: u16) -> u8 {
        let match_addr = (address - ECHO_RAM_OFFSET);
        match match_addr {
            WRAM_BANK_00_START..=WRAM_BANK_00_END => self.read_bank_0(match_addr),
            WRAM_BANK_NN_START..=WRAM_BANK_NN_END => self.read_bank_n(match_addr),
            _ => panic!("Disallowed EchoRam read: 0x{:04X}", match_addr),
        }
    }

    pub fn write_bank_0(&mut self, address: u16, value: u8) {
        self.memory[(address - WRAM_OFFSET) as usize] = value;
    }

    pub fn write_bank_n(&mut self, address: u16, value: u8) {
        let address = self.bank_selected * WRAM_BANK_SIZE + (address - WRAM_BANK_N_OFFSET) as usize;
        self.memory[address] = value;
    }

    pub fn write_echo_ram(&mut self, address: u16, value: u8) {
        let match_addr = (address - ECHO_RAM_OFFSET);
        match match_addr {
            WRAM_BANK_00_START..=WRAM_BANK_00_END => self.write_bank_0(match_addr, value),
            WRAM_BANK_NN_START..=WRAM_BANK_NN_END => self.write_bank_n(match_addr, value),
            _ => panic!("Disallowed EchoRam write: 0x{:04X}", match_addr),
        }
    }
}