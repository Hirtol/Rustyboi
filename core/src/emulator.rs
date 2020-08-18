use crate::hardware::HardwareOwner;
use crate::hardware::cpu::CPU;
use crate::hardware::memory::Memory;
use crate::io::bootrom::*;
use crate::hardware::ppu::PPU;
use crate::hardware::cartridge::Cartridge;

pub struct Emulator {
    cpu: CPU,
    mmu: Memory,
    ppu: PPU,
    boot_rom: BootRom,
    cartridge: Cartridge,
}

impl HardwareOwner for Emulator {
    fn read_byte(&mut self, address: u16) -> u8 {
        self.mmu.read_byte(address)
    }
}
