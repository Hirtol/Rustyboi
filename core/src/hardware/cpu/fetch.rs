//! Purely here to provide an extra implementation block so that the main mod.rs doesn't get
//! too cluttered.

use crate::hardware::cpu::instructions::Instruction;
use crate::hardware::cpu::CPU;

impl CPU {
    /// Fetches the next instruction.
    /// Modifies the `opcode` value, as well as advances the `PC` as necessary
    pub fn get_next_instruction(&mut self) -> Instruction {
        self.opcode = self.memory.read_byte(self.registers.pc);
        let instruction;

        if self.opcode != 0xCB {
            instruction = Instruction::decode(self.opcode);
        } else {
            self.registers.pc.wrapping_add(1);
            instruction = Instruction::decode_prefix(self.memory.read_byte(self.registers.pc + 1));
        }

        self.registers.pc.wrapping_add(1);

        instruction
    }

    /// Based on the current `PC` will interpret the value at the location in memory as a `u8`
    /// value.
    /// Advances the `PC` by 1.
    pub fn get_instr_u8(&mut self) -> u8 {
        let result = self.memory.read_byte(self.registers.pc);
        self.registers.pc.wrapping_add(1);

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
}
