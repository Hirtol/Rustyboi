pub const MEMORY_SIZE: usize = 0xFFFF;
pub const IO_START_ADDRESS: u16 = 0xFF00;

#[derive(Debug)]
pub struct Memory {
    memory: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            memory: vec![0u8; MEMORY_SIZE],
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    pub fn set_byte(&mut self, address: u16, value: u8) {
        //TODO: Add bound checks to ensure we're not accessing protected memory.
        self.memory[address as usize] = value;
    }

    pub fn read_short(&self, address: u16) -> u16 {
        let least_s_byte = self.read_byte(address) as u16;
        let most_s_byte = self.read_byte(address.wrapping_add(1)) as u16;

        (most_s_byte << 8) | least_s_byte
    }

    pub fn set_short(&mut self, address: u16, value: u16) {
        self.set_byte(address, (value & 0xFF) as u8); // Least significant byte first.
        self.set_byte(address.wrapping_add(1), ((value & 0xFF00) >> 8) as u8);
    }
}
