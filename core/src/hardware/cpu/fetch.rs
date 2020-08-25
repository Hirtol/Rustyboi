//! Purely here to provide an extra implementation block so that the main mod.rs doesn't get
//! too cluttered.

use crate::hardware::cpu::instructions::Instruction;
use crate::hardware::cpu::CPU;
use crate::hardware::memory::MemoryMapper;

impl<M: MemoryMapper> CPU<M> {
    /// Fetches the next instruction.
    /// Modifies the `opcode` value, as well as advances the `PC` as necessary
    ///
    /// # Returns
    ///
    /// * `true` if the instruction is a prefix instruction, `false` otherwise.
    pub fn read_next_instruction(&mut self) -> bool {
        self.opcode = self.get_instr_u8();

        if self.opcode == 0xCB {
            self.opcode = self.get_instr_u8();
            true
        } else {
            false
        }
    }

    /// Based on the current `PC` will interpret the value at the location in memory as a `u8`
    /// value.
    /// Advances the `PC` by 1.
    pub fn get_instr_u8(&mut self) -> u8 {
        let result = self.read_byte_cycle(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);

        result
    }

    /// Based on the current `PC` will interpret the `current` and `current + 1` byte at those locations
    /// in memory as a `u16` value resolved as little endian (least significant byte first).
    /// Advances the `PC` by 2.
    pub fn get_instr_u16(&mut self) -> u16 {
        let least_s_byte = self.get_instr_u8() as u16;
        let most_s_byte = self.get_instr_u8() as u16;

        (most_s_byte << 8) | least_s_byte
    }

    /// Read a byte from the `MMU` and increment the cycle counter by 4.
    pub fn read_byte_cycle(&mut self, address: u16) -> u8 {
        // Every memory fetch costs 4 cycles (could divide by 4 as well)
        self.cycles_performed += 4;
        self.mmu.borrow().read_byte(address)
    }

    /// Set a byte in the `MMU` and increment the cycle counter by 4.
    pub fn write_byte_cycle(&mut self, address: u16, value: u8) {
        self.cycles_performed += 4;
        self.mmu.borrow_mut().write_byte(address, value);
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
        self.write_byte_cycle(address.wrapping_add(1), ((value & 0xFF00) >> 8) as u8);
    }
}
