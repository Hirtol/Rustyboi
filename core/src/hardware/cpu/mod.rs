use crate::hardware::cpu::execute::{InstructionAddress, JumpModifier, WrapperEnum};
use crate::hardware::cpu::instructions::*;
use crate::hardware::cpu::traits::{SetU16, SetU8, ToU16, ToU8};
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

    /// Pass 4 ticks
    fn nop(&mut self) {
        return;
    }

    /// `ld   rr,nn       x1 nn nn  12 ---- rr=nn (rr may be BC,DE,HL or SP)`
    /// OR
    /// `ld   SP,HL       F9         8 ---- SP=HL`
    fn load_16bit<T: Copy, U: Copy>(&mut self, destination: T, source: U)
    where
        Self: SetU16<T>,
        Self: ToU16<U>,
    {
        let source_value = self.get_u16_value(source);

        self.set_u16_value(destination, source_value);
    }

    /// `ld` never sets any flags.
    fn load_8bit<T: Copy, U: Copy>(&mut self, destination: T, source: U)
    where
        Self: SetU8<T>,
        Self: ToU8<U>,
    {
        let source_value = self.read_u8_value(source);

        self.set_u8_value(destination, source_value);
    }

    /// `r=r+1` OR `(HL)=(HL)+1`
    ///
    /// Flags: `z0h-`
    fn increment<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let old_value = self.read_u8_value(target);
        let new_value = old_value.wrapping_add(1);

        self.registers.set_zf(new_value == 0);
        self.registers.set_n(false);
        self.registers.set_h((old_value & 0xF) + 0x1 > 0xF);

        self.set_u8_value(target, new_value);
    }

    /// `rr = rr+1      ;rr may be BC,DE,HL,SP`
    ///
    /// Flags: `----`
    fn increment16(&mut self, target: Reg16) {
        let new_value = self.get_u16_value(target).wrapping_add(1);

        self.set_u16_value(target, new_value);
    }

    /// `rotate A left; 7th bit to Carry flag`
    ///
    /// Flags: `000c` (current) OR `z00c`
    fn rlca(&mut self) {
        let carry_bit = self.registers.a & 0x80 != 0;
        //TODO: Check if ZF should be set here, conflicting documentation.
        self.registers.set_zf(false);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf(carry_bit);

        self.registers.a = self.registers.a.rotate_left(1);
    }

    /// `HL = HL+rr     ;rr may be BC,DE,HL,SP`
    ///
    /// Flags: `-0hc`
    fn add_16bit(&mut self, target: Reg16) {
        let old_value = self.get_u16_value(target);
        let (result, overflowed) = old_value.overflowing_add(self.registers.hl());
        self.registers.set_n(false);
        self.registers.set_cf(overflowed);
        self.registers
            .set_h((old_value & 0x0FFF) + (self.registers.hl() & 0x0FFF) > 0x0FFF);

        self.registers.set_hl(result);
    }

    /// `r=r-1` OR `(HL)=(HL)-1`
    ///
    /// Flags: `z1h-`
    fn decrement<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let old_value = self.read_u8_value(target);
        let new_value = old_value.wrapping_sub(1);

        self.registers.set_zf(new_value == 0);
        self.registers.set_n(true);
        //TODO: Check half carry flag for decrement
        self.registers.set_h(old_value & 0xF == 0);

        self.set_u8_value(target, new_value);
    }

    /// `rr = rr-1      ;rr may be BC,DE,HL,SP`
    ///
    /// Flags: `----`
    fn decrement16(&mut self, target: Reg16) {
        let new_value = self.get_u16_value(target).wrapping_sub(1);

        self.set_u16_value(target, new_value);
    }

    /// `Rotate A right. Old bit 0 to Carry flag.`
    ///
    /// Flags: `000C`
    fn rrca(&mut self) {
        let carry_bit = self.registers.a & 0x01 != 0;
        //TODO: Check if ZF should be set here, conflicting documentation.
        self.registers.set_zf(false);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf(carry_bit);

        self.registers.a = self.registers.a.rotate_right(1);
    }

    /// low power standby mode (VERY low power)
    fn stop(&mut self) {
        //TODO: Implement (Interrupts?)
        unimplemented!("STOP called, implement!");
    }

    /// Rotate A left through Carry flag.
    ///
    /// Flags: `000C`
    fn rla(&mut self) {
        let carry_bit = self.registers.a & 0x80;
        let new_value = (self.registers.a.wrapping_shl(1)) | self.registers.cf() as u8;

        self.registers.set_zf(false);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf(carry_bit != 0);

        self.registers.a = new_value;
    }

    /// `jr   PC+dd` OR `jr   f,PC+dd`
    ///
    /// Add n to current address and jump to it.
    /// Conditional relative jump if nz,z,nc,c.
    ///
    /// Flags: `----`
    fn relative_jump(&mut self, condition: JumpModifier) {
        if self.matches_jmp_condition(condition) {
            let offset = self.get_instr_u8() as i8;
            // No idea why this works, but wrapping_sub/add depending on negative/positive value
            // always caused an addition even when casting to u16.
            self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
        } else {
            self.registers.pc = self.registers.pc.wrapping_add(1);
        }
    }

    /// Rotate A right through Carry flag.
    ///
    /// Flags: `000C`
    fn rra(&mut self) {
        let carry_bit = self.registers.a & 0x01;
        let new_value = ((self.registers.cf() as u8) << 7) | (self.registers.a.wrapping_shr(1));

        self.registers.set_zf(false);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf(carry_bit != 0);

        self.registers.a = new_value;
    }

    /// Decimal adjust register A.
    /// This instruction adjusts register A so that the
    /// correct representation of Binary Coded Decimal (BCD) is obtained.
    /// Used [this] for the implementation as it was a rather confusing instruction.
    ///
    /// Flags: `Z-0C`
    ///
    /// [this]: https://forums.nesdev.com/viewtopic.php?t=15944#:~:text=The%20DAA%20instruction%20adjusts%20the,%2C%20lower%20nybble%2C%20or%20both.
    fn daa(&mut self) {
        // after an addition, adjust if (half-)carry occurred or if result is out of bounds
        if !self.registers.n() {
            if self.registers.cf() || self.registers.a > 0x99 {
                self.registers.a = self.registers.a.wrapping_add(0x60);
                self.registers.set_cf(true);
            }
            if self.registers.hf() || (self.registers.a & 0x0F) > 0x09 {
                self.registers.a = self.registers.a.wrapping_add(0x06);
            }
        }
        else {
            // after a subtraction, only adjust if (half-)carry occurred
            if self.registers.cf() {
                self.registers.a = self.registers.a.wrapping_sub(0x60);
            }
            if self.registers.hf() {
                self.registers.a = self.registers.a.wrapping_sub(0x06);
            }
        }

        self.registers.set_zf(self.registers.a == 0);
        self.registers.set_h(false);
    }

    /// ComPLement accumulator (A = ~A).
    ///
    /// Flags: `-11-`
    fn cpl(&mut self) {
        self.registers.a = !self.registers.a;

        self.registers.set_n(true);
        self.registers.set_h(true);
    }

    /// Set Carry Flag.
    ///
    /// Flags: `-001`
    fn scf(&mut self) {
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf(true);
    }

    /// Complement Carry Flag.
    ///
    /// Flags: `-00i` where `i = inverted`
    fn ccf(&mut self) {
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.f.toggle(Flags::CF);
    }

    /// `halt until interrupt occurs (low power)`
    fn halt(&mut self) {
        //TODO: Finish implementing this.
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
        let value = self.read_u8_value(target);
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

    ///Add the value in `target` plus the carry flag to A.
    ///
    /// Flags: `Z0HC`
    fn adc<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
        let value = self.read_u8_value(target) + self.registers.cf() as u8;
        let (new_value, overflowed) = self.registers.a.overflowing_add(value);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(false);
        self.registers.set_cf(overflowed);
        self.registers
            .set_h((self.registers.a & 0xF) + (value & 0xF) > 0xF);

        self.registers.a = new_value;
    }

    /// Subtract the value in `target` from A.
    ///
    /// Flags: `Z1HC`
    fn sub<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
        let value = self.read_u8_value(target);
        let (new_value, overflowed) = self.registers.a.overflowing_sub(value);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(true);
        self.registers.set_cf(overflowed);
        //TODO: Check if works
        self.registers
            .set_h(((self.registers.a & 0xF) - (value & 0xF)) < 0);

        self.registers.a = new_value;
    }

    /// Subtract the value in `target` and the carry flag from A.
    ///
    /// Flags: `Z1HC`
    fn sbc<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
        let value = self.read_u8_value(target) + self.registers.cf() as u8;
        let (new_value, overflowed) = self.registers.a.overflowing_sub(value);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(true);
        self.registers.set_cf(overflowed);
        //TODO: Check if works
        self.registers
            .set_h(((self.registers.a & 0xF) - (value & 0xF)) < 0);

        self.registers.a = new_value;
    }

    /// Bitwise AND between the value in `target` and A.
    ///
    /// Flags: `Z010`
    fn and<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
        self.registers.a &= self.read_u8_value(target);

        self.registers.set_zf(self.registers.a == 0);
        self.registers.set_n(false);
        self.registers.set_h(true);
        self.registers.set_cf(false);
    }

    /// Bitwise XOR between the value in `target` and A.
    ///
    /// Flags: `Z000`
    fn xor<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
        self.registers.a ^= self.read_u8_value(target);

        self.registers.set_zf(self.registers.a == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf(false);
    }

    /// Store into A the bitwise OR of the value in `target` and A.
    ///
    /// Flags: `Z000`
    fn or<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
        self.registers.a |= self.read_u8_value(target);

        self.registers.set_zf(self.registers.a == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf(false);
    }

    /// Subtract the value in `target` from A and set flags accordingly, but don't store the result.
    /// This is useful for ComParing values.
    ///
    /// Flags: `Z1HC`
    fn compare<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
        let value = self.read_u8_value(target);
        let (new_value, overflowed) = self.registers.a.overflowing_sub(value);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(true);
        self.registers.set_cf(overflowed);
        //TODO: Check if works
        self.registers
            .set_h(((self.registers.a & 0xF) - (value & 0xF)) < 0);
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
                //TODO: Consider moving to seperate function to clean up enum.
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
            JumpModifier::NotZero => !self.registers.zf(),
            JumpModifier::Zero => self.registers.zf(),
            JumpModifier::NotCarry => !self.registers.cf(),
            JumpModifier::Carry => self.registers.cf(),
            JumpModifier::Always => true,
            JumpModifier::HL => true,
        }
    }

    fn call(&mut self, target: JumpModifier) {}

    fn push(&mut self, target: Reg16) {}

    fn rst(&mut self, numb: u8) {}

    /// There are a few instructions in the GameBoy's instruction set which are not used.
    /// For now we'll panic, but it may be that some games call them erroneously, so consider
    /// just returning instead.
    fn unknown(&mut self) {
        panic!("Unknown function was called, opcode: {}", self.opcode)
    }

    fn reti(&mut self) {}

    fn add_sp(&mut self) {}

    fn di(&mut self) {}

    fn load_sp(&mut self) {}

    fn ei(&mut self) {}

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
    fn read_u8_value(&mut self, target: Reg8) -> u8 {
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
    fn set_u8_value(&mut self, target: Reg8, value: u8) {
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
    fn read_u8_value(&mut self, target: InstructionAddress) -> u8 {
        use crate::hardware::memory::IO_START_ADDRESS;
        use InstructionAddress::*;

        match target {
            BCI => self.memory.read_byte(self.registers.bc()),
            DEI => self.memory.read_byte(self.registers.de()),
            HLI => self.memory.read_byte(self.registers.hl()),
            HLIP => {
                let result = self.read_u8_value(HLI);
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                result
            }
            HLIN => {
                let result = self.read_u8_value(HLI);
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                result
            }
            DIRECT => self.get_instr_u8(),
            DirectMem => {
                let address = self.get_instr_u16();
                self.memory.read_byte(address)
            }
            IoDirect => {
                let address = self.get_instr_u8() as u16;
                self.memory.read_byte(IO_START_ADDRESS + address)
            }
            IoC => self
                .memory
                .read_byte(IO_START_ADDRESS + self.registers.c as u16),
        }
    }
}

impl SetU8<InstructionAddress> for CPU {
    fn set_u8_value(&mut self, target: InstructionAddress, value: u8) {
        use crate::hardware::memory::IO_START_ADDRESS;
        use InstructionAddress::*;

        match target {
            BCI => self.memory.set_byte(self.registers.bc(), value),
            DEI => self.memory.set_byte(self.registers.de(), value),
            HLI => self.memory.set_byte(self.registers.hl(), value),
            HLIP => {
                self.set_u8_value(HLI, value);
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
            }
            HLIN => {
                self.set_u8_value(HLI, value);
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
            IoC => self
                .memory
                .set_byte(IO_START_ADDRESS + self.registers.c as u16, value),
        }
    }
}

impl ToU8<WrapperEnum> for CPU {
    fn read_u8_value(&mut self, target: WrapperEnum) -> u8 {
        match target {
            WrapperEnum::Reg8(result) => self.read_u8_value(result),
            WrapperEnum::InstructionAddress(result) => self.read_u8_value(result),
        }
    }
}

impl SetU8<WrapperEnum> for CPU {
    fn set_u8_value(&mut self, target: WrapperEnum, value: u8) {
        match target {
            WrapperEnum::Reg8(result) => self.set_u8_value(result, value),
            WrapperEnum::InstructionAddress(result) => self.set_u8_value(result, value),
        }
    }
}

impl ToU8<u8> for CPU {
    fn read_u8_value(&mut self, target: u8) -> u8 {
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
        use crate::hardware::memory::IO_START_ADDRESS;
        use InstructionAddress::*;

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
        use crate::hardware::memory::IO_START_ADDRESS;
        use InstructionAddress::*;

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
            }
            DirectMem => {
                let address = self.get_instr_u16();
                self.memory.set_short(address, value);
            }
            IoDirect => unimplemented!(),
            IoC => unimplemented!(),
        }
    }
}
