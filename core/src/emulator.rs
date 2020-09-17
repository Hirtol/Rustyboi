use std::rc::Rc;

use bitflags::_core::cell::RefCell;


use crate::hardware::cpu::CPU;

use crate::hardware::memory::{Memory};
use crate::hardware::ppu::palette::{DmgColor};
use crate::hardware::ppu::{FRAMEBUFFER_SIZE, PPU};


use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::io::joypad::*;

/// A DMG runs at `4.194304 MHz` with a Vsync of `59.7275 Hz`, so that would be
/// `4194304 / 59.7275 = 70224 cycles/frame`
pub const CYCLES_PER_FRAME: u32 = 70224;

pub type MMU<T> = Rc<RefCell<T>>;

pub struct Emulator {
    cpu: CPU<Memory>,
}

impl Emulator {
    pub fn new(boot_rom: Option<[u8; 256]>, cartridge: &[u8]) -> Self {
        Emulator {
            cpu: CPU::new(Memory::new(boot_rom, cartridge, PPU::new())),
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
    pub fn frame_buffer(&self) -> [DmgColor; FRAMEBUFFER_SIZE] {
        self.cpu.mmu.ppu.frame_buffer().clone()
    }

    /// Emulate one CPU cycle, and any other things that need to happen.
    ///
    /// # Returns
    ///
    /// The delta in clock cycles due to the current emulation, to be used
    /// for timing purposes by the consumer of the emulator.
    pub fn emulate_cycle(&mut self) -> u64 {
        let mut prior_cycles = self.cpu.cycles_performed;

        self.handle_interrupts();

        //TODO: Consider ticking PPU/timers here since interrupts do tick timers.?

        self.cpu.step_cycle();

        let delta_cycles = self.cpu.cycles_performed - prior_cycles;

        let mut interrupt = self.cpu.mmu.ppu.do_cycle(delta_cycles as u32);
        self.add_new_interrupts(interrupt);

        interrupt = self.cpu.mmu.timers.tick_timers(delta_cycles);
        self.add_new_interrupts(interrupt);

        // For PPU timing, maybe see how many cycles the cpu did, pass this to the PPU,
        // and have the PPU run until it has done all those, OR reaches an interrupt.
        // Need some way to remember the to be done cycles then though.
        // EI checker? Run till EI is enabled sort of thing.
        delta_cycles
    }

    /// Pass the provided `InputKey` to the emulator and ensure it's `pressed` state
    /// is represented for the current running `ROM`.
    pub fn handle_input(&mut self, input: InputKey, pressed: bool) {
        let result = self.handle_external_input(input, pressed);
        self.add_new_interrupts(result);
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

    /// Add any inputs to the existing flag register, will as a side effect stop the CPU from
    /// HALTing.
    fn add_new_interrupts(&mut self, interrupt: Option<InterruptFlags>) {
        if let Some(intr) = interrupt {
            //log::trace!("Adding interrupt: {:?}", intr);
            let mut interrupts = self.get_interrupts();
            interrupts.insert(intr);
            self.cpu.mmu.interrupts_flag = interrupts;
        }
    }

    fn handle_interrupts(&mut self) {
        let mut interrupt_flags: InterruptFlags = self.get_interrupts();

        if !self.cpu.ime {
            // While we have interrupts pending we can't enter halt mode again.
            if !interrupt_flags.is_empty(){
                self.cpu.halted = false;
            }
            return;
        }else if interrupt_flags.is_empty() {
            return;
        }

        let interrupt_enable: InterruptFlags = self.cpu.mmu.interrupts_enable;

        // Thanks to the iterator this should go in order, therefore also giving us the proper
        // priority. This is not at all optimised, so consider changing this for a better performing
        // version. Something without bitflags mayhap.
        for interrupt in Interrupts::iter() {
            let repr_flag = InterruptFlags::from_bits_truncate(interrupt as u8);
            if !(repr_flag & interrupt_flags & interrupt_enable).is_empty() {
                //log::debug!("Firing {:?} interrupt", interrupt);
                interrupt_flags.remove(repr_flag);

                self.cpu.mmu.interrupts_flag = interrupt_flags;

                self.cpu.interrupts_routine(interrupt);
                // We disable IME after an interrupt routine, thus we should preemptively break this loop.
                break;
            }
        }
    }

    fn get_interrupts(&self) -> InterruptFlags {
        self.cpu.mmu.interrupts_flag
    }
}

// pub fn tilemap_image(&self) {
//     //let tile_data: TileData = self.cpu.mmu.get_tile_data();
//     let back_x = 8;
//     let back_y = 1024;
//
//     let mut imagebuf = image::ImageBuffer::new(back_x, back_y);
//
//     for (x, y, pixel) in imagebuf.enumerate_pixels_mut() {
//         let dx = 7 - (x % 8);
//         let dy = 2 * (y % 8) as u16;
//         // Pixel data is spread over 2 bytes
//         let a = self.cpu.mmu.read_byte((0x8200 + y) as u16);
//         let b = self.cpu.mmu.read_byte((0x8200 + y + 1) as u16);
//         //warn!("READING BYTES: {:04x} {:04x}", 0x9000 + y, 0x9000 +y + 1);
//         let bit1 = (a & (1 << dx)) >> dx;
//         let bit2 = (b & (1 << dx)) >> dx;
//
//         let pixeldata = bit1 | (bit2 << 1);
//         let color = self
//             .mmu
//             .borrow()
//             .ppu
//             .colorisor
//             .get_color(&self.cpu.mmu.ppu.bg_window_palette.color(pixeldata));
//
//         *pixel = image::Rgb([color.0, color.1, color.2]);
//     }
//
//     imagebuf.save("test.png").unwrap();
// }
