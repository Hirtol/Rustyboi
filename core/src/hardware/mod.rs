pub mod cpu;
pub mod memory;
pub mod ppu;
pub mod registers;
pub mod cartridge;

pub trait HardwareOwner {
    fn read_byte(&mut self, address: u16) -> u8;
}