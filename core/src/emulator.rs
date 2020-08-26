use std::rc::Rc;

use bitflags::_core::cell::RefCell;
use log::*;

use crate::hardware::cpu::CPU;
use crate::hardware::HardwareOwner;
use crate::hardware::memory::{Memory, MemoryMapper};
use crate::hardware::ppu::PPU;
use crate::io::bootrom::*;

pub const CYCLES_PER_FRAME: u32 = 70221;

pub type MMU<T> = Rc<RefCell<T>>;

pub struct Emulator {
    cpu: CPU<Memory>,
    mmu: MMU<Memory>,
    ppu: PPU,
}

impl Emulator {
    pub fn new(boot_rom: Option<[u8; 256]>, cartridge: &[u8]) -> Self {
        let mmu = MMU::new(RefCell::new(Memory::new(boot_rom, cartridge)));
        Emulator {
            cpu: CPU::new(&mmu),
            mmu,
            ppu: PPU {},
        }
    }

    pub fn cycles_performed(&self) -> u128 {
        self.cpu.cycles_performed
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
