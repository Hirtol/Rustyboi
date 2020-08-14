
const MEMORY_SIZE: usize = 0xFFFF;

#[derive(Default, Debug)]
pub struct Memory {
    memory: [u8; MEMORY_SIZE],
}

impl Memory {
    pub fn read_byte(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    pub fn set_byte(&mut self, address: u16, value: u8) {
        //TODO: Add bound checks to ensure we're not accessing protected memory.
        self.memory[address as usize] = value;
    }
}