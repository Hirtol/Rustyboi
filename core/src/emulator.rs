use std::rc::Rc;

use bitflags::_core::cell::RefCell;

use crate::hardware::cpu::CPU;

use crate::hardware::mmu::{Memory, MemoryMapper};
use crate::hardware::ppu::palette::DmgColor;
use crate::hardware::ppu::{FRAMEBUFFER_SIZE, PPU};

use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::io::joypad::*;
use crate::EmulatorOptions;

/// A DMG runs at `4.194304 MHz` with a Vsync of `59.7275 Hz`, so that would be
/// `4194304 / 59.7275 = 70224 cycles/frame`
pub const CYCLES_PER_FRAME: u64 = 70224;

/// Describes the `Emulator`'s mode.
///
/// If DMG is chosen no CGB features will be used.
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub enum EmulatorMode {
    DMG,
    CGB,
}

impl EmulatorMode {
    pub fn is_dmg(&self) -> bool {
        *self == EmulatorMode::DMG
    }

    pub fn is_cgb(&self) -> bool {
        *self == EmulatorMode::CGB
    }
}

pub struct Emulator {
    cpu: CPU<Memory>,
}

impl Emulator {
    pub fn new(cartridge: &[u8], options: EmulatorOptions) -> Self {
        Emulator {
            cpu: CPU::new(Memory::new(cartridge, options)),
        }
    }

    /// Return how many cycles the CPU has performed so far.
    ///
    /// Mainly useful for timing.
    pub fn cycles_performed(&self) -> u64 {
        self.cpu.cycles_performed
    }

    /// Returns the current `frame buffer` from the `PPU`.
    ///
    /// Should only be called on multiples of [CYCLES_PER_FRAME](constant.CYCLES_PER_FRAME.html)
    /// otherwise the data will be only partially complete.
    pub fn frame_buffer(&self) -> &[DmgColor; FRAMEBUFFER_SIZE] {
        self.cpu.mmu.ppu.frame_buffer()
    }

    pub fn audio_buffer(&self) -> &[f32] {
        self.cpu.mmu.apu.get_audio_buffer()
    }

    pub fn clear_audio_buffer(&mut self) {
        self.cpu.mmu.apu.clear_audio_buffer();
    }

    /// Returns, if the current `ROM` has a battery, the contents of the External Ram.
    ///
    /// Should be used for saving functionality.
    pub fn battery_ram(&self) -> Option<&[u8]> {
        self.cpu.mmu.cartridge()?.mbc().get_battery_ram()
    }

    pub fn game_title(&self) -> Option<&str> {
        Some(self.cpu.mmu.cartridge()?.cartridge_header().title.as_str())
    }

    /// Emulate one CPU cycle, and any other things that need to happen.
    ///
    /// # Returns
    ///
    /// The delta in clock cycles due to the current emulation, to be used
    /// for timing purposes by the consumer of the emulator.
    ///
    /// Also returns whether VBlank occurred in this emulator cycle.
    pub fn emulate_cycle(&mut self) -> (u64, bool) {
        let mut prior_cycles = self.cpu.cycles_performed;

        self.cpu.step_cycle();

        (self.cpu.cycles_performed - prior_cycles, self.cpu.added_vblank())
    }

    /// Pass the provided `InputKey` to the emulator and ensure it's `pressed` state
    /// is represented for the current running `ROM`.
    pub fn handle_input(&mut self, input: InputKey, pressed: bool) {
        let result = self.handle_external_input(input, pressed);
        self.cpu.mmu.add_new_interrupts(result);
    }

    fn handle_external_input(&mut self, input: InputKey, pressed: bool) -> Option<InterruptFlags> {
        let inputs = &mut self.cpu.mmu.joypad_register;

        if pressed {
            inputs.press_key(input);
            Some(InterruptFlags::JOYPAD)
        } else {
            inputs.release_key(input);
            None
        }
    }
}
