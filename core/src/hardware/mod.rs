use crate::hardware::memory::Memory;
use crate::hardware::ppu::PPU;
use crate::io::bootrom::BootRom;
use crate::hardware::cartridge::Cartridge;

pub mod cartridge;
pub mod cpu;
pub mod memory;
pub mod ppu;
pub mod registers;

pub struct Hardware {
    mmu: Memory,
    ppu: PPU,
    boot_rom: BootRom,
    cartridge: Cartridge,
}

pub trait HardwareOwner {
    fn read_byte(&mut self, address: u16) -> u8;
}
