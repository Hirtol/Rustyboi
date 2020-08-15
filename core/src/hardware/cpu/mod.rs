use crate::hardware::cpu::execute::{InstructionAddress, JumpModifier, WrapperEnum};
use crate::hardware::cpu::instructions::*;
use crate::hardware::cpu::traits::{SetU8, ToU16, ToU8};
use crate::hardware::memory::Memory;
use crate::hardware::registers::{Flags, Reg16, Reg8, Registers};
use log::*;
use std::io::Read;

#[cfg(test)]
mod tests;

mod execute;
mod fetch;
mod instructions;
mod traits;

pub struct CPU {
    opcode: u8,
    registers: Registers,
    memory: Memory,
    halted: bool,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            opcode: 0,
            registers: Registers::new(),
            memory: Memory::new(),
            halted: false,
        }
    }

    /// Fetches the next instruction and executes it as well.
    pub fn step_cycle(&mut self) {
        if self.halted {
            return;
        }

        let is_prefix = self.read_next_instruction();

        trace!(
            "Executing opcode: {} - prefixed: {}",
            self.opcode,
            is_prefix
        );

        if is_prefix {
            self.execute_prefix(self.opcode);
        } else {
            self.execute(self.opcode);
        }
    }

    fn load_8bit<T: Copy, U: Copy>(&mut self, destination: T, source: U)
    where
        Self: SetU8<T>,
        Self: ToU8<U>,
    {
        let source_value = self.get_reg_value(source);

        self.set_value(destination, source_value);
    }

    /// `halt until interrupt occurs (low power)`
    fn halt(&mut self) {
        self.halted = true;
    }

    /// `A=A+r` OR `A=A+n` OR `A=A+(HL)`
    /// Adds the provided `target` to the `A` register, setting any relevant flags.
    ///
    /// # Arguments
    ///
    /// * `target` - The value to be added, a relevant `ToU8` implementation should exist for `CPU`
    fn add<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
        let value = self.get_reg_value(target);
        let (new_value, overflowed) = self.registers.a.overflowing_add(value);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(false);
        self.registers.set_cf(overflowed);
        // Half Carry is set if adding the lower nibbles of the value and register A
        // together result in a value bigger than 0xF. If the result is larger than 0xF
        // than the addition caused a carry from the lower nibble to the upper nibble.
        self.registers
            .set_h((self.registers.a & 0xF) + (value & 0xF) > 0xF);

        self.registers.a = new_value;
    }

    fn adc<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn sub<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn sbc<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn and<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn xor<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn or<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn compare<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn ret(&mut self, target: JumpModifier) {}

    fn pop(&mut self, target: Reg16) {}

    /// `jump to nn, PC=nn` OR `jump to HL, PC=HL` OR `conditional jump if nz,z,nc,c`
    /// Sets the `PC` to the relevant value based on the JumpCondition
    ///
    /// # Arguments
    ///
    /// * `condition` - The `JumpCondition` to be evaluated for the jump.
    fn jump(&mut self, condition: JumpModifier) {
        //TODO: Timing.
        if self.matches_jmp_condition(condition) {
            let target_address: u16 = if let JumpModifier::HL = condition {
                self.registers.hl()
            } else {
                self.get_instr_u16()
            };

            self.registers.pc = target_address;
        } else {
            // 3 byte wide instruction, we by default increment 1.
            // Therefore we still need to increment by 2.
            self.registers.pc.wrapping_add(2);
        }
    }

    fn matches_jmp_condition(&self, condition: JumpModifier) -> bool {
        match condition {
            JumpModifier::NotZero => !self.registers.f.contains(Flags::ZF),
            JumpModifier::Zero => self.registers.f.contains(Flags::ZF),
            JumpModifier::NotCarry => !self.registers.f.contains(Flags::CF),
            JumpModifier::Carry => self.registers.f.contains(Flags::CF),
            JumpModifier::Always => true,
            JumpModifier::HL => true,
        }
    }

    fn call(&mut self, target: JumpModifier) {}

    fn push(&mut self, target: Reg16) {}

    /*
       Prefixed Instructions
    */

    fn rlc<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn rrc<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn rl<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn rr<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn sla<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn sra<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn swap<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn srl<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn bit<T: Copy>(&mut self, bit: u8, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn set<T: Copy>(&mut self, bit: u8, target: T)
    where
        Self: ToU8<T>,
    {
    }

    fn res<T: Copy>(&mut self, bit: u8, target: T)
    where
        Self: ToU8<T>,
    {
    }
}

impl ToU8<Reg8> for CPU {
    fn get_reg_value(&mut self, target: Reg8) -> u8 {
        use Reg8::*;

        match target {
            A => self.registers.a,
            B => self.registers.b,
            C => self.registers.c,
            D => self.registers.d,
            E => self.registers.e,
            H => self.registers.h,
            L => self.registers.l,
        }
    }
}

impl SetU8<Reg8> for CPU {
    fn set_value(&mut self, target: Reg8, value: u8) {
        use Reg8::*;

        match target {
            A => self.registers.a = value,
            B => self.registers.b = value,
            C => self.registers.c = value,
            D => self.registers.d = value,
            E => self.registers.e = value,
            H => self.registers.h = value,
            L => self.registers.l = value,
        }
    }
}

impl ToU8<InstructionAddress> for CPU {
    fn get_reg_value(&mut self, target: InstructionAddress) -> u8 {
        use InstructionAddress::*;
        //TODO: Finish
        match target {
            BCI => 0,
            DEI => 0,
            HLI => self.memory.read_byte(self.registers.hl()),
            HLIP => 0,
            HLID => 0,
            ADDR => self.get_instr_u8(),
        }
    }
}

impl SetU8<InstructionAddress> for CPU {
    fn set_value(&mut self, target: InstructionAddress, value: u8) {
        use InstructionAddress::*;
        //TODO: Finish
        match target {
            BCI => {}
            DEI => {}
            HLI => self.memory.set_byte(self.registers.hl(), value),
            HLIP => {}
            HLID => {}
            ADDR => {
                let address = self.get_instr_u16();
                self.memory.set_byte(address, value)
            }
        }
    }
}

impl ToU8<WrapperEnum> for CPU {
    fn get_reg_value(&mut self, target: WrapperEnum) -> u8 {
        match target {
            WrapperEnum::Reg8(result) => self.get_reg_value(result),
            WrapperEnum::InstructionAddress(result) => self.get_reg_value(result),
        }
    }
}

impl SetU8<WrapperEnum> for CPU {
    fn set_value(&mut self, target: WrapperEnum, value: u8) {
        match target {
            WrapperEnum::Reg8(result) => self.set_value(result, value),
            WrapperEnum::InstructionAddress(result) => self.set_value(result, value),
        }
    }
}

impl ToU8<u8> for CPU {
    fn get_reg_value(&mut self, target: u8) -> u8 {
        target
    }
}
