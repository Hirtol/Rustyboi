use crate::hardware::registers::{Registers, Reg8};
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
        CPU{
            opcode: 0,
            registers: Default::default(),
            memory: Memory::new(),
        }
    }

    pub fn step_cycle(&mut self) {
        self.opcode = self.memory.read_byte(self.registers.pc);
        let instruction = Instruction::decode(self.opcode);

        trace!("Executing opcode: {} - {:?}", self.opcode, instruction);

        self.execute(instruction);

        self.registers.pc += 1;
    }

    pub fn execute(&mut self, instruction: Instruction) {
        match instruction {
            Instruction::ADD(target) => self.add(target),
            Instruction::SUB(target) => self.sub(target),
            _ => debug!("Unimplemented instruction: {:?}", instruction)
        }
    }

    /// `A=A+r` OR `A=A+n` OR `A=A+(HL)`
    /// Adds the provided `target` to the `A` register, setting any relevant flags
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
            HL => panic!("NOT IMPLEMENTED HL GETTER"), //TODO: Implement
            _ => panic!("Unimplemented target: {:?}", target)
        }
    }
}