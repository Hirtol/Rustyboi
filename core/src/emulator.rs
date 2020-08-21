use crate::hardware::HardwareOwner;
use crate::hardware::cpu::CPU;
use crate::hardware::memory::Memory;
use crate::io::bootrom::*;
use crate::hardware::ppu::PPU;
use crate::hardware::cartridge::Cartridge;
use std::rc::Rc;
use bitflags::_core::cell::RefCell;
use log::*;

pub type MMU = Rc<RefCell<Memory>>;

pub struct Emulator {
    cpu: CPU,
    mmu: MMU,
    ppu: PPU,
}

impl Emulator {
    pub fn new(boot_rom: Option<[u8; 256]>, cartridge: &[u8]) -> Self {
        let mmu = MMU::new(RefCell::new(Memory::new(boot_rom, cartridge)));
        Emulator { cpu: CPU::new(&mmu), mmu, ppu: PPU {} }
    }

    pub fn emulate_cycle(&mut self) {
        self.cpu.step_cycle();
    }
}



impl HardwareOwner for Emulator {
    fn read_byte(&mut self, address: u16) -> u8 {
        self.mmu.borrow().read_byte(address)
    }
}
