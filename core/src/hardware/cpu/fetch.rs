//! Purely here to provide an extra implementation block so that the main mod.rs doesn't get
//! too cluttered.

use crate::hardware::cpu::CPU;
use crate::hardware::memory::MemoryMapper;
use crate::io::interrupts::{InterruptFlags, Interrupts};

impl<M: MemoryMapper> CPU<M> {
    /// Add 4 cycles to the internal counter
    pub fn add_cycles(&mut self) {
        self.cycles_performed += 4;
        let mut interrupt = self.mmu.ppu_mut().do_cycle(4);
        self.add_new_interrupts(interrupt);

        interrupt = self.mmu.timers_mut().tick_timers();
        self.add_new_interrupts(interrupt);

        self.mmu.apu_mut().tick(4);
    }

    /// Read the next opcode, advance the PC, and call the execute function for
    /// a prefix opcode.
    pub fn cb_prefix_call(&mut self) {
        self.opcode = self.get_instr_u8();
        // log::trace!(
        //     "Executing opcode: {:04X} - registers: {}",
        //     self.opcode,
        //     self.registers,
        // );
        self.execute_prefix(self.opcode);
    }

    /// Retrieves the next opcode and advances the PC.
    ///
    /// If there is a pending interrupt the opcode fetch will be aborted mid-query (so after 4 ticks
    /// have already passed for the memory fetch), then the interrupt routine will be launched and
    /// the opcode at the interrupt routine location returned.
    pub fn get_next_opcode(&mut self) -> u8 {
        // If we have enabled interrupts pending.
        // if !(self.mmu.interrupts().interrupt_flag & self.mmu.interrupts().interrupt_enable).is_empty() {
        //     self.add_cycles();
        // }
        let mut opcode = self.get_instr_u8();
        if self.handle_interrupts() {
            opcode = self.get_instr_u8();
        }

        opcode
    }

    pub fn handle_interrupts(&mut self) -> bool{
        let mut interrupt_flags: InterruptFlags = self.mmu.interrupts().interrupt_flag;

        if !self.ime {
            // While we have interrupts pending we can't enter halt mode again.
            if !(interrupt_flags & self.mmu.interrupts().interrupt_enable).is_empty() {
                self.halted = false;
                self.add_cycles();
            }
            return false;
        } else if interrupt_flags.is_empty() {
            return false;
        }

        let interrupt_enable: InterruptFlags = self.mmu.interrupts().interrupt_enable;

        // Thanks to the iterator this should go in order, therefore also giving us the proper
        // priority. This is not at all optimised, so consider changing this for a better performing
        // version. Something without bitflags mayhap.
        for interrupt in Interrupts::iter() {
            let repr_flag = InterruptFlags::from_bits_truncate(interrupt as u8);
            if !(repr_flag & interrupt_flags & interrupt_enable).is_empty() {
                //log::debug!("Firing {:?} interrupt", interrupt);
                interrupt_flags.remove(repr_flag);

                self.mmu.interrupts_mut().interrupt_flag = interrupt_flags;
                self.registers.pc -= 1;
                self.interrupts_routine(interrupt);
                // We disable IME after an interrupt routine, thus we should preemptively break this loop.
                //break;
                return true;
            }
        }
        false
    }

    fn handle_interrupt_quircks(&mut self) -> bool{
        // While we have interrupts pending we can't enter halt mode again.
        if !self.ime {
            self.halted = false;
            self.add_cycles();
            true
        } else {
            false
        }
    }

    /// Based on the current `PC` will interpret the value at the location in memory as a `u8`
    /// value.
    ///
    /// Advances the `PC` by 1.
    pub fn get_instr_u8(&mut self) -> u8 {
        let result = self.read_byte_cycle(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);

        result
    }

    /// Based on the current `PC` will interpret the `current` and `current + 1` byte at those locations
    /// in memory as a `u16` value resolved as little endian (least significant byte first).
    ///
    /// Advances the `PC` by 2.
    pub fn get_instr_u16(&mut self) -> u16 {
        let least_s_byte = self.get_instr_u8() as u16;
        let most_s_byte = self.get_instr_u8() as u16;

        (most_s_byte << 8) | least_s_byte
    }

    /// Read a byte from the `MMU` and increment the cycle counter by 4.
    pub fn read_byte_cycle(&mut self, address: u16) -> u8 {
        self.add_cycles();
        self.mmu.read_byte(address)
    }

    /// Set a byte in the `MMU` and increment the cycle counter by 4.
    pub fn write_byte_cycle(&mut self, address: u16, value: u8) {
        self.add_cycles();
        self.mmu.write_byte(address, value);
        //TODO: Potentially add DMA transfer cycles (160*4 cycles) here?
    }

    /// Read a `short` in the `MMU` and increment the cycle counter by 8.
    pub fn read_short_cycle(&mut self, address: u16) -> u16 {
        let least_s_byte = self.read_byte_cycle(address) as u16;
        let most_s_byte = self.read_byte_cycle(address.wrapping_add(1)) as u16;

        (most_s_byte << 8) | least_s_byte
    }

    /// Set a `short` in the `MMU` and increment the cycle counter by 8.
    pub fn write_short_cycle(&mut self, address: u16, value: u16) {
        self.write_byte_cycle(address, (value & 0xFF) as u8); // Least significant byte first.
        self.write_byte_cycle(address.wrapping_add(1), (value >> 8) as u8);
    }

    /// Temporary hack to see if we rendered `VBlank` this execution cycle.
    ///
    /// Resets `VBlank` to `false` if it was `true`.
    pub fn added_vblank(&mut self) -> bool {
        if self.had_vblank {
            self.had_vblank = false;
            true
        }else {
            false
        }
    }

    /// Add a new interrupt to the IF flag.
    pub fn add_new_interrupts(&mut self, interrupt: Option<InterruptFlags>) {
        if let Some(intr) = interrupt {
            if intr.contains(InterruptFlags::VBLANK) {
                self.had_vblank = true;
            }
            self.mmu.interrupts_mut().insert_interrupt(intr);
        }
    }


}
