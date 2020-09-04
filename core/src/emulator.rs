use std::rc::Rc;

use bitflags::_core::cell::RefCell;
use log::*;

use crate::hardware::cpu::CPU;
use crate::hardware::memory::*;
use crate::hardware::memory::{Memory, MemoryMapper};
use crate::hardware::ppu::palette::DisplayColour;
use crate::hardware::ppu::tiledata::TileData;
use crate::hardware::ppu::{FRAMEBUFFER_SIZE, PPU};
use crate::hardware::HardwareOwner;
use crate::io::bootrom::*;
use crate::io::interrupts::Interrupts::VBLANK;
use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::io::joypad::*;

/// A DMG runs at `4.194304 MHz` with a Vsync of `59.7275 Hz`, so that would be
/// `4194304 / 59.7275 = 70224 cycles/frame`
pub const CYCLES_PER_FRAME: u32 = 70224;

pub type MMU<T> = Rc<RefCell<T>>;

pub struct Emulator {
    cpu: CPU<Memory>,
    mmu: MMU<Memory>,
}

impl Emulator {
    pub fn new(
        boot_rom: Option<[u8; 256]>,
        cartridge: &[u8],
        display_colors: DisplayColour,
    ) -> Self {
        let mmu = MMU::new(RefCell::new(Memory::new(
            boot_rom,
            cartridge,
            PPU::new(display_colors),
        )));
        Emulator {
            cpu: CPU::new(&mmu),
            mmu,
        }
    }

    /// Return how many cycles the CPU has performed so far.
    ///
    /// Mainly useful for timing.
    pub fn cycles_performed(&self) -> u128 {
        self.cpu.cycles_performed
    }

    pub fn frame_buffer(&self) -> [u8; FRAMEBUFFER_SIZE] {
        self.mmu.borrow().ppu.frame_buffer().clone()
    }

    /// Emulate one CPU cycle, and any other things that need to happen.
    ///
    /// # Returns
    ///
    /// The delta in clock cycles due to the current emulation, to be used
    /// for timing purposes by the consumer of the emulator.
    pub fn emulate_cycle(&mut self) -> u128 {
        self.handle_interrupts();

        let prior_cycles = self.cpu.cycles_performed;

        self.cpu.step_cycle();

        let delta_cycles = self.cpu.cycles_performed - prior_cycles;

        let mut interrupt = self.mmu.borrow_mut().ppu.do_cycle(delta_cycles as u32);
        self.add_new_interrupts(interrupt);
        interrupt = self.mmu.borrow_mut().timers.tick_timers(delta_cycles);
        self.add_new_interrupts(interrupt);

        // For PPU timing, maybe see how many cycles the cpu did, pass this to the PPU,
        // and have the PPU run until it has done all those, OR reaches an interrupt.
        // Need some way to remember the to be done cycles then though.
        // EI checker? Run till EI is enabled sort of thing.
        delta_cycles
    }

    /// Pass the provided `InputKey` to the emulator and ensure it's `pressed` state
    /// is represented for the current running `ROM`.
    pub fn handle_input(&mut self, input: InputKey) {
        self.add_new_interrupts(self.handle_external_input(input));
    }

    fn handle_external_input(&self, input: InputKey) -> Option<InterruptFlags> {
        let mut inputs = &mut self.mmu.borrow_mut().joypad_register;

        debug!("Setting Handle for input {:?}", input);

        match input {
            InputKey::START(pressed) => {
                inputs.set(JoypadFlags::BUTTON_KEYS, !pressed);
                inputs.set(JoypadFlags::DOWN_START, !pressed);
            },
            InputKey::SELECT(pressed) => {
                inputs.set(JoypadFlags::BUTTON_KEYS, !pressed);
                inputs.set(JoypadFlags::UP_SELECT, !pressed);
            },
            InputKey::A(pressed) => {
                inputs.set(JoypadFlags::BUTTON_KEYS, !pressed);
                inputs.set(JoypadFlags::RIGHT_A, !pressed);
            },
            InputKey::B(pressed) => {
                inputs.set(JoypadFlags::BUTTON_KEYS, !pressed);
                inputs.set(JoypadFlags::LEFT_B, !pressed);
            },
            InputKey::UP(pressed) => {
                inputs.set(JoypadFlags::DIRECTION_KEYS, !pressed);
                inputs.set(JoypadFlags::UP_SELECT, !pressed);
            },
            InputKey::DOWN(pressed) => {
                inputs.set(JoypadFlags::DIRECTION_KEYS, !pressed);
                inputs.set(JoypadFlags::DOWN_START, !pressed);
            },
            InputKey::LEFT(pressed) => {
                inputs.set(JoypadFlags::DIRECTION_KEYS, !pressed);
                inputs.set(JoypadFlags::LEFT_B, !pressed);
            },
            InputKey::RIGHT(pressed) => {
                inputs.set(JoypadFlags::DIRECTION_KEYS, !pressed);
                inputs.set(JoypadFlags::RIGHT_A, !pressed);
            },
        }

        Some(InterruptFlags::JOYPAD)
    }

    /// Add any inputs to the existing flag register, will as a side effect stop the CPU from
    /// HALTing.
    fn add_new_interrupts(&mut self, interrupt: Option<InterruptFlags>) {
        if let Some(intr) = interrupt {
           self.cpu.halted = false;

            let mut interrupts = self.get_interrupts();
            interrupts.insert(intr);
            self.mmu.borrow_mut().write_byte(INTERRUPTS_FLAG, interrupts.bits())
        }
    }

    fn handle_interrupts(&mut self) {
        if !self.cpu.ime {
            return;
        }

        let mut interrupt_flags: InterruptFlags = self.get_interrupts();

        let interrupt_enable: InterruptFlags =
            InterruptFlags::from_bits_truncate(self.mmu.borrow().read_byte(INTERRUPTS_ENABLE));

        if interrupt_flags.is_empty() {
            return;
        }

        // Thanks to the iterator this should go in order, therefore also giving us the proper
        // priority. This is not at all optimised, so consider changing this for a better performing
        // version. Something without bitflags mayhap.
        for interrupt in Interrupts::iter() {
            let repr_flag = InterruptFlags::from_bits_truncate(interrupt as u8);

            if interrupt_flags.contains(repr_flag) && interrupt_enable.contains(repr_flag) {
                debug!("Firing {:?} interrupt", interrupt);
                interrupt_flags.remove(repr_flag);

                self.mmu
                    .borrow_mut()
                    .write_byte(INTERRUPTS_FLAG, interrupt_flags.bits());
                self.cpu.interrupts_routine(interrupt);
            }
        }
    }

    fn get_interrupts(&self) -> InterruptFlags {
        InterruptFlags::from_bits_truncate(self.mmu.borrow().read_byte(INTERRUPTS_FLAG))
    }
}

// pub fn tilemap_image(&self) {
//     //let tile_data: TileData = self.mmu.borrow().get_tile_data();
//     let back_x = 8;
//     let back_y = 1024;
//
//     let mut imagebuf = image::ImageBuffer::new(back_x, back_y);
//
//     for (x, y, pixel) in imagebuf.enumerate_pixels_mut() {
//         let dx = 7 - (x % 8);
//         let dy = 2 * (y % 8) as u16;
//         // Pixel data is spread over 2 bytes
//         let a = self.mmu.borrow().read_byte((0x8200 + y) as u16);
//         let b = self.mmu.borrow().read_byte((0x8200 + y + 1) as u16);
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
//             .get_color(&self.mmu.borrow().ppu.bg_window_palette.color(pixeldata));
//
//         *pixel = image::Rgb([color.0, color.1, color.2]);
//     }
//
//     imagebuf.save("test.png").unwrap();
// }
