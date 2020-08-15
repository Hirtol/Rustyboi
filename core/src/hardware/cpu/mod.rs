use crate::hardware::registers::{Registers, Reg8, Flags};
use crate::hardware::cpu::instructions::*;
use log::*;
use std::io::Read;
use crate::hardware::cpu::traits::ToU8;
use crate::hardware::memory::Memory;

#[cfg(test)]
mod tests;

mod instructions;
mod traits;

struct CPU {
    opcode: u8,
    registers: Registers,
    memory: Memory,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            opcode: 0,
            registers: Registers::new(),
            memory: Memory::new(),
        }
    }

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
    pub fn get_instr_u8(&mut self) -> u8{
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

    pub fn step_cycle(&mut self) {
        let instruction = self.get_next_instruction();

        trace!("Executing opcode: {} - {:?}", self.opcode, instruction);

        self.execute(instruction);
    }

    /// Execute the provided Instruction, note this does *not* automatically increment the `PC`
    /// unless done so by an instruction itself.
    pub fn execute(&mut self, instruction: Instruction) {
        match instruction {
            Instruction::ADD(target) => self.add(target),
            Instruction::SUB(target) => self.sub(target),
            Instruction::JP(condition) => self.jump(condition),
            _ => debug!("Unimplemented instruction: {:?}", instruction)
        }
    }

    /// `A=A+r` OR `A=A+n` OR `A=A+(HL)`
    /// Adds the provided `target` to the `A` register, setting any relevant flags.
    ///
    /// # Arguments
    ///
    /// * `target` - The value to be added, a relevant `ToU8` implementation should exist for `CPU`
    fn add<T: Copy>(&mut self, target: T)
        where Self: ToU8<T> {
        let value = self.get_reg_value(target);
        let (new_value, overflowed) = self.registers.a.overflowing_add(value);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(false);
        self.registers.set_cf(overflowed);
        // Half Carry is set if adding the lower nibbles of the value and register A
        // together result in a value bigger than 0xF. If the result is larger than 0xF
        // than the addition caused a carry from the lower nibble to the upper nibble.
        self.registers.set_h((self.registers.a & 0xF) + (value & 0xF) > 0xF);

        self.registers.a = new_value;
    }

    /// `jump to nn, PC=nn` OR `jump to HL, PC=HL` OR `conditional jump if nz,z,nc,c`
    /// Sets the `PC` to the relevant value based on the JumpCondition
    ///
    /// # Arguments
    ///
    /// * `condition` - The `JumpCondition` to be evaluated for the jump.
    fn jump(&mut self, condition: JumpCondition) {
        //TODO: Timing.
        if self.matches_jmp_condition(condition) {
            let target_address;

            if let JumpCondition::HL = condition{
                target_address = self.registers.hl();
            }else{
                let least_s_byte = self.memory.read_byte(self.registers.pc) as u16;
                let most_s_byte = self.memory.read_byte(self.registers.pc + 1) as u16;

                target_address = (most_s_byte << 8) | least_s_byte;
            }

            self.registers.pc = target_address;
        } else {
            // 3 byte wide instruction, we by default increment 1.
            // Therefore we still need to increment by 2.
            self.registers.pc.wrapping_add(2);
        }
    }

    fn matches_jmp_condition(&self, condition: JumpCondition) -> bool {
        match condition {
            JumpCondition::NotZero => !self.registers.f.contains(Flags::ZF),
            JumpCondition::Zero => self.registers.f.contains(Flags::ZF),
            JumpCondition::NotCarry => !self.registers.f.contains(Flags::CF),
            JumpCondition::Carry => self.registers.f.contains(Flags::CF),
            JumpCondition::Always => true,
            JumpCondition::HL => true,
        }
    }

    fn sub<T: Copy>(&mut self, target: T)
        where Self: ToU8<T> {}
}

impl ToU8<RegistryTarget> for CPU {
    fn get_reg_value(&self, target: RegistryTarget) -> u8 {
        use RegistryTarget::*;

        match target {
            A => self.registers.a,
            B => self.registers.b,
            C => self.registers.c,
            D => self.registers.d,
            E => self.registers.e,
            H => self.registers.h,
            L => self.registers.l,
            HL => self.memory.read_byte(self.registers.hl()),
            _ => panic!("Unimplemented target: {:?}", target)
        }
    }
}

impl ToU8<u8> for CPU {
    fn get_reg_value(&self, target: u8) -> u8 {
        target
    }
}