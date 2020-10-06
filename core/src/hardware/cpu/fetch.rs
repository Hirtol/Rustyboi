//! Purely here to provide an extra implementation block so that the main mod.rs doesn't get
//! too cluttered.

use crate::hardware::cpu::CPU;
use crate::hardware::mmu::MemoryMapper;
use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::scheduler::{Event, EventType};

impl<M: MemoryMapper> CPU<M> {
    /// Add 4 cycles to the internal counter
    pub fn add_cycles(&mut self) {
        self.cycles_performed += 4;

        if self.mmu.do_m_cycle() {
            self.had_vblank = true;
        }
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
        let mut opcode = self.read_byte_cycle(self.registers.pc);

        if self.handle_interrupts() {
            opcode = self.read_byte_cycle(self.registers.pc);
        }

        self.registers.pc = self.registers.pc.wrapping_add(1);

        opcode
    }

    pub fn handle_interrupts(&mut self) -> bool {
        if !self.ime {
            if self.mmu.interrupts().interrupts_pending() {
                self.halted = false;
                self.add_cycles();
            }
        } else if self.mmu.interrupts().interrupts_pending() {
            let interrupt = self.mmu.interrupts().get_immediate_interrupt();
            //log::debug!("Firing {:?} interrupt", interrupt);
            self.mmu.interrupts_mut().interrupt_flag.remove(interrupt);

            self.interrupts_routine(interrupt);

            return true;
        }
        false
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
        } else {
            false
        }
    }
}
