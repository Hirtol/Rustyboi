use std::rc::Rc;

use bitflags::_core::cell::RefCell;
use log::*;

use crate::hardware::cpu::CPU;
use crate::hardware::HardwareOwner;
use crate::hardware::memory::{Memory, MemoryMapper};
use crate::hardware::ppu::PPU;
use crate::hardware::memory::*;
use crate::io::bootrom::*;
use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::io::interrupts::Interrupts::VBLANK;

/// A DMG runs at `4.194304 MHz` with a Vsync of `59.73 Hz`, so that would be
/// `4194304 / 59.73 ~= 70221 cycles/frame`
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
            ppu: PPU { frame_buffer: [0; crate::hardware::ppu::FRAMEBUFFER_SIZE] },
        }
    }

    /// Return how many cycles the CPU has performed so far.
    ///
    /// Mainly useful for timing.
    pub fn cycles_performed(&self) -> u128 {
        self.cpu.cycles_performed
    }

    /// Emulate one CPU cycle, and any other things that need to happen.
    pub fn emulate_cycle(&mut self) {
        self.handle_interrupts();

        self.cpu.step_cycle();
    }

    pub fn frame_buffer(&self) -> &[u8] {
        &[0u8; crate::hardware::ppu::FRAMEBUFFER_SIZE]
    }

    fn handle_interrupts(&mut self) {
        if !self.cpu.ime {
            return;
        }

        let mut interrupt_flags: InterruptFlags = InterruptFlags::from_bits_truncate(self.mmu.borrow().read_byte(INTERRUPTS_FLAG));
        let interrupt_enable: InterruptFlags = InterruptFlags::from_bits_truncate(self.mmu.borrow().read_byte(INTERRUPTS_ENABLE));

        if interrupt_flags.is_empty(){
            return;
        }

        // Thanks to the iterator this should go in order, therefore also giving us the proper
        // priority. This is not at all optimised, so consider changing this for a better performing
        // version. Something without bitflags mayhap.
        for interrupt in Interrupts::iter() {
            let repr_flag = InterruptFlags::from_bits_truncate(interrupt as u8);

            if interrupt_flags.contains(repr_flag) && interrupt_enable.contains(repr_flag) {
                trace!("Firing {:?} interrupt", interrupt);
                interrupt_flags.remove(repr_flag);

                self.mmu.borrow_mut().write_byte(INTERRUPTS_FLAG, interrupt_flags.bits());
                self.cpu.interrupts_routine(interrupt);
            }
        }
    }
}
