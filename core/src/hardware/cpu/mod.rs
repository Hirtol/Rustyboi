use crate::hardware::cpu::execute::{InstructionAddress, JumpModifier, WrapperEnum};
use crate::hardware::cpu::instructions::*;
use crate::hardware::cpu::traits::{SetU16, SetU8, ToU16, ToU8};
use crate::hardware::memory::Memory;
use crate::hardware::registers::Reg8::A;
use crate::hardware::registers::{Flags, Reg16, Reg8, Registers};
use log::*;
use std::io::Read;
use std::rc::Rc;
use bitflags::_core::cell::RefCell;
use crate::emulator::*;

#[cfg(test)]
mod tests;

mod alu;
mod execute;
mod fetch;
mod instructions;
mod traits;

#[derive(Debug)]
pub struct CPU {
    opcode: u8,
    registers: Registers,
    mmu: MMU,
    halted: bool,
}

impl CPU {
    pub fn new(mmu: &MMU) -> Self {
        CPU {
            opcode: 0,
            registers: Registers::new(),
            mmu: mmu.clone(),
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
        let source_value = self.read_u16_value(source);

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
        let new_value = self.read_u16_value(target).wrapping_add(1);

        self.set_u16_value(target, new_value);
    }

    /// `rotate A left; 7th bit to Carry flag`
    ///
    /// Flags: `000c` (current) OR `z00c`
    fn rlca(&mut self) {
        self.rotate_left(A);
        self.registers.set_zf(false);
    }

    /// `HL = HL+rr     ;rr may be BC,DE,HL,SP`
    ///
    /// Flags: `-0hc`
    fn add_16bit(&mut self, target: Reg16) {
        let old_value = self.read_u16_value(target);
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
        let new_value = self.read_u16_value(target).wrapping_sub(1);

        self.set_u16_value(target, new_value);
    }

    /// `Rotate A right. Old bit 0 to Carry flag.`
    ///
    /// Flags: `000C`
    fn rrca(&mut self) {
        self.rotate_right(A);

        self.registers.set_zf(false);
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
        self.rotate_left_carry(A);
        self.registers.set_zf(false);
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
        self.rotate_right_carry(A);
        self.registers.set_zf(false);
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
        } else {
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
    /// Flags: `Z0HC`
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
            .set_h(((self.registers.a & 0xF) < (value & 0xF)));

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
            .set_h(((self.registers.a & 0xF) < (value & 0xF)));

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
            .set_h(((self.registers.a & 0xF) < (value & 0xF)));
    }

    /// Return from subroutine.
    /// This is basically a POP PC (if such an instruction existed).
    fn ret(&mut self, target: JumpModifier) {
        if self.matches_jmp_condition(target) {
            self.registers.pc = self.mmu.borrow_mut().read_short(self.registers.sp);
            self.registers.sp = self.registers.sp.wrapping_add(2);
        }
    }

    /// Pop register `target` from the stack.
    ///
    /// Flags: `----`
    fn pop(&mut self, target: Reg16) {
        let sp_target = self.mmu.borrow_mut().read_short(self.registers.sp);
        self.set_u16_value(target, sp_target);
        self.registers.sp = self.registers.sp.wrapping_add(2);
    }

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

    /// Call address n16, if condition `target` is met.
    /// This pushes the address of the instruction after the CALL on the stack,
    /// such that RET can pop it later;
    /// then, it executes an implicit JP n16.
    ///
    /// Flags: `----`
    fn call(&mut self, target: JumpModifier) {
        if self.matches_jmp_condition(target) {
            let address = self.get_instr_u16();
            self.push_helper(self.registers.pc);
            self.registers.pc = address;
        } else {
            self.registers.pc = self.registers.pc.wrapping_add(2);
        }
    }

    /// Push register `target` into the stack.
    ///
    /// Flags: `----`
    fn push(&mut self, target: Reg16) {
        let value = self.read_u16_value(target);
        self.push_helper(value);
    }

    /// Helper function to push certain values to the stack.
    fn push_helper(&mut self, value: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.mmu.borrow_mut().set_short(self.registers.sp, value);
    }

    /// Call address `vec`.
    /// This is a shorter and faster equivalent to `CALL` for suitable values of `vec`.
    ///
    /// Flags: `----`
    fn rst(&mut self, vec: u8) {
        self.push_helper(self.registers.pc);
        self.registers.pc = vec as u16;
    }

    /// There are a few instructions in the GameBoy's instruction set which are not used.
    /// For now we'll panic, but it may be that some games call them erroneously, so consider
    /// just returning instead.
    fn unknown(&mut self) {
        panic!("Unknown function was called, opcode: {}", self.opcode)
    }

    /// Return from subroutine and enable interrupts.
    /// This is basically equivalent to executing EI then RET,
    /// meaning that IME is set right after this instruction.
    ///
    /// Flags: `----`
    fn reti(&mut self) {
        //TODO: Once we have interrupts.
        unimplemented!("RETI NOT YET IMPLEMENTED");
    }

    /// `ADD SP,e8`
    /// Add the signed value e8 to SP.
    ///
    /// Flags: `00HC`
    fn add_sp(&mut self) {
        let value = self.get_instr_u8() as i8;
        let (new_value, overflowed) = self.registers.sp.overflowing_add(value as u16);
        self.registers.set_zf(false);
        self.registers.set_n(false);
        //TODO: Check if this half flag is correct (doc says bit 3, but this should be a 16 bit?!)
        self.registers
            .set_h((self.registers.sp & 0xF) + (value as u16 & 0xF) > 0xF);
        self.registers.set_cf(overflowed);

        self.registers.sp = new_value;
    }

    /// `DI`
    /// Disable Interrupts by clearing the IME flag.
    ///
    /// Flags: `----`
    fn di(&mut self) {
        //TODO: Implement interrupts.
        unimplemented!("RETI NOT YET IMPLEMENTED");
    }

    /// `LD HL,SP+i8`
    /// Load the value of `SP + i8` into the register `HL`.
    ///
    /// Flags: `00HC`
    fn load_sp(&mut self) {
        let value = self.get_instr_u8() as i8;
        let (new_value, overflowed) = self.registers.sp.overflowing_add(value as u16);
        self.registers.set_hl(new_value);

        self.registers.set_zf(false);
        self.registers.set_n(false);
        self.registers
            .set_h((self.registers.sp & 0xF) + (value as u16 & 0xF) > 0xF);
        self.registers.set_cf(overflowed);
    }

    /// `EI`
    /// Enable Interrupts by setting the IME flag.
    /// The flag is only set after the instruction following EI.
    fn ei(&mut self) {
        //TODO: Implement interrupts.
        unimplemented!("RETI NOT YET IMPLEMENTED");
    }

    /*
       Prefixed Instructions
    */

    /// `RLC r8/[HL]`
    /// Rotate register `target` left.
    ///
    /// C <- [7 <- 0] <- [7]
    ///
    /// Flags: `Z00C`
    fn rlc<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        self.rotate_left(target);
    }

    /// `RRC r8/[HL]`
    /// Rotate register r8 right.
    ///
    /// [0] -> [7 -> 0] -> C
    ///
    /// Flags: `Z00C`
    fn rrc<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        self.rotate_right(target);
    }

    /// `RL r8/[HL]`
    /// Rotate bits in register `target` left through carry.
    ///
    /// C <- [7 <- 0] <- C
    ///
    /// Flags: `Z00C`
    fn rl<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        self.rotate_left_carry(target);
    }

    /// `RR r8/[HL]`
    /// Rotate register `target` right through carry.
    ///
    /// C -> [7 -> 0] -> C
    ///
    /// Flags: `Z00C`
    fn rr<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        self.rotate_right_carry(target);
    }

    /// `SLA r8/[HL]`
    /// Shift Left Arithmetic on register `target`.
    ///
    /// C <- [7 <- 0] <- 0
    ///
    /// Flags: `Z00C`
    fn sla<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        self.shift_left(target);
    }

    /// `SRA r8/[HL]`
    /// Shift Right Arithmetic register `target`.
    ///
    /// [7] -> [7 -> 0] -> C
    ///
    /// Flags: `Z00C`
    fn sra<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let new_value = (value & 0x80) | value.wrapping_shr(1);

        self.registers.set_zf(new_value == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf((value & 0x1) != 0);

        self.set_u8_value(target, new_value);
    }

    /// `SWAP r8/[HL]`
    /// Swap upper 4 bits in register `target` and the lower 4 ones.
    ///
    /// Flags: `Z000`
    fn swap<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let new_value = ((value & 0x0F) << 4) | ((value & 0xF0) >> 4);

        self.registers.set_zf(new_value == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf(false);

        self.set_u8_value(target, new_value);
    }

    /// `SRL r8/[HL]`
    /// Shift Right Logic register `target`.
    ///
    /// 0 -> [7 -> 0] -> C
    ///
    /// Flags: `Z00C`
    fn srl<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        self.shift_right(target);
    }

    /// `BIT u3,r8/[HL]`
    /// Test bit u3 in register `target`, set the zero flag if bit not set.
    ///
    /// Flags: `Z01-`
    fn bit<T: Copy>(&mut self, bit: u8, target: T)
    where
        Self: ToU8<T>,
    {
        let value = self.read_u8_value(target);
        let bitmask = 1 << bit;

        self.registers.set_zf((value & bitmask) == 0);
        self.registers.set_n(false);
        self.registers.set_h(true)
    }

    /// `SET u3,r8/[HL]`
    /// Set bit u3 in register r8 to 1.
    /// Bit 0 is the rightmost one, bit 7 the leftmost one.
    ///
    /// Flags: `----`
    fn set<T: Copy>(&mut self, bit: u8, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let bitmask: u8 = 1 << bit;

        self.set_u8_value(target, value | bitmask);
    }

    /// `RES u3,r8/[HL]`
    /// Set bit u3 in register r8 to 0.
    /// Bit 0 is the rightmost one, bit 7 the leftmost one.
    ///
    /// Flags: `----`
    fn res<T: Copy>(&mut self, bit: u8, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let bit_mask: u8 = 0x1 << bit;

        self.set_u8_value(target, value & !bit_mask);
    }
}
