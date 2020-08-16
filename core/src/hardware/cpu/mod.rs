use crate::hardware::cpu::execute::{InstructionAddress, JumpModifier, WrapperEnum};
use crate::hardware::cpu::instructions::*;
use crate::hardware::cpu::traits::{SetU8, ToU16, ToU8, SetU16};
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

    fn nop(&mut self) {
        return;
    }

    fn load_16bit<T: Copy, U: Copy>(&mut self, destination: T, source: U)
        where
            Self: SetU16<T>,
            Self: ToU16<U>,
    {
        let source_value = self.get_u16_value(source);

        self.set_u16_value(destination, source_value);
    }

    fn load_8bit<T: Copy, U: Copy>(&mut self, destination: T, source: U)
        where
            Self: SetU8<T>,
            Self: ToU8<U>,
    {
        let source_value = self.get_reg_value(source);

        self.set_value(destination, source_value);
    }

    /// `r=r+1` OR `(HL)=(HL)+1`
    fn increment<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {

    }

    /// `rr = rr+1      ;rr may be BC,DE,HL,SP`
    ///
    /// Flags: ----
    fn increment16(&mut self, target: Reg16){

    }

    /// `rotate akku left`
    fn rlca(&mut self){

    }

    /// `HL = HL+rr     ;rr may be BC,DE,HL,SP`
    fn add_16bit<T: Copy>(&mut self, target: T)
        where
        Self: ToU16<T>,
    {

    }

    /// `r=r-1` OR `(HL)=(HL)-1`
    fn decrement<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {

    }

    /// `rr = rr-1      ;rr may be BC,DE,HL,SP`
    ///
    /// Flags: ----
    fn decrement16(&mut self, target: Reg16){

    }

    /// `rotate akku right`
    fn rrca(&mut self){

    }

    /// low power standby mode (VERY low power)
    fn stop(&mut self){

    }

    fn rla(&mut self){

    }

    fn relative_jump(&mut self, condition: JumpModifier) {

    }

    fn rra(&mut self){

    }

    fn daa(&mut self){

    }

    fn cpl(&mut self){

    }

    fn scf(&mut self){

    }

    fn ccf(&mut self){

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
    {}

    fn sub<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn sbc<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn and<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn xor<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn or<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn compare<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

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
            self.registers.pc = self.registers.pc.wrapping_add(2);
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

    fn rst(&mut self, numb: u8){

    }

    /// There are a few instructions in the GameBoy's instruction set which are not used.
    /// For now we'll panic, but it may be some games call them erroneously, so consider
    /// just returning instead.
    fn unknown(&mut self){
        panic!("Unknown function was called, opcode: {}", self.opcode)
    }

    fn reti(&mut self){

    }

    fn add_sp(&mut self){

    }

    fn di(&mut self) {

    }

    fn load_sp(&mut self){

    }

    fn ei(&mut self){

    }


    /*
       Prefixed Instructions
    */

    fn rlc<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn rrc<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn rl<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn rr<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn sla<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn sra<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn swap<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn srl<T: Copy>(&mut self, target: T)
        where
            Self: ToU8<T>,
    {}

    fn bit<T: Copy>(&mut self, bit: u8, target: T)
        where
            Self: ToU8<T>,
    {}

    fn set<T: Copy>(&mut self, bit: u8, target: T)
        where
            Self: ToU8<T>,
    {}

    fn res<T: Copy>(&mut self, bit: u8, target: T)
        where
            Self: ToU8<T>,
    {}
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
        use crate::hardware::memory::IO_START_ADDRESS;

        match target {
            BCI => self.memory.read_byte(self.registers.bc()),
            DEI => self.memory.read_byte(self.registers.de()),
            HLI => self.memory.read_byte(self.registers.hl()),
            HLIP => {
                let result = self.get_reg_value(HLI);
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                result
            },
            HLIN => {
                let result = self.get_reg_value(HLI);
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                result
            },
            DIRECT => self.get_instr_u8(),
            DirectMem => {
                let address = self.get_instr_u16();
                self.memory.read_byte(address)
            },
            IoDirect => {
                let address = self.get_instr_u8() as u16;
                self.memory.read_byte(IO_START_ADDRESS + address)
            },
            IoC => self.memory.read_byte(IO_START_ADDRESS + self.registers.c as u16),
        }
    }
}

impl SetU8<InstructionAddress> for CPU {
    fn set_value(&mut self, target: InstructionAddress, value: u8) {
        use InstructionAddress::*;
        use crate::hardware::memory::IO_START_ADDRESS;

        match target {
            BCI => self.memory.set_byte(self.registers.bc(), value),
            DEI => self.memory.set_byte(self.registers.de(), value),
            HLI => self.memory.set_byte(self.registers.hl(), value),
            HLIP => {
                self.set_value(HLI, value);
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
            }
            HLIN => {
                self.set_value(HLI, value);
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
            }
            DIRECT => {
                let address = self.get_instr_u16();
                self.memory.set_byte(address, value)
            }
            DirectMem => {
                let address = self.get_instr_u16();
                self.memory.set_byte(address, value)
            }
            IoDirect => {
                let addition = self.get_instr_u8() as u16;
                self.memory.set_byte(IO_START_ADDRESS + addition, value);
            }
            IoC => self.memory.set_byte(IO_START_ADDRESS + self.registers.c as u16, value),
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

impl ToU16<Reg16> for CPU {
    fn get_u16_value(&mut self, target: Reg16) -> u16 {
        use Reg16::*;

        match target {
            AF => self.registers.af(),
            BC => self.registers.bc(),
            DE => self.registers.de(),
            HL => self.registers.hl(),
            SP => self.registers.sp,
        }
    }
}

impl SetU16<Reg16> for CPU {
    fn set_u16_value(&mut self, target: Reg16, value: u16) {
        use Reg16::*;

        match target {
            AF => self.registers.set_af(value),
            BC => self.registers.set_bc(value),
            DE => self.registers.set_de(value),
            HL => self.registers.set_hl(value),
            SP => self.registers.sp = value,
        }
    }
}

impl ToU16<InstructionAddress> for CPU {
    fn get_u16_value(&mut self, target: InstructionAddress) -> u16 {
        use InstructionAddress::*;
        use crate::hardware::memory::IO_START_ADDRESS;

        match target {
            BCI => unimplemented!(),
            DEI => unimplemented!(),
            HLI => unimplemented!(),
            HLIP => unimplemented!(),
            HLIN => unimplemented!(),
            DIRECT => self.get_instr_u16(),
            DirectMem => unimplemented!(),
            IoDirect => unimplemented!(),
            IoC => unimplemented!(),
        }
    }
}

impl SetU16<InstructionAddress> for CPU {
    fn set_u16_value(&mut self, target: InstructionAddress, value: u16) {
        use InstructionAddress::*;
        use crate::hardware::memory::IO_START_ADDRESS;

        match target {
            BCI => unimplemented!(),
            DEI => unimplemented!(),
            HLI => unimplemented!(),
            HLIP => unimplemented!(),
            HLIN => unimplemented!(),
            DIRECT => {
                //TODO: Check if big endian/little endian
                let address = self.get_instr_u16();
                self.memory.set_short(address, value);
                // self.memory.set_byte(address, (value & 0x0F) as u8);
                // self.memory.set_byte(address.wrapping_add(1), (value & 0xF0 >> 8) as u8);
            },
            DirectMem => {
                let address = self.get_instr_u16();
                self.memory.set_short(address, value);
            },
            IoDirect => unimplemented!(),
            IoC => unimplemented!(),
        }
    }
}
