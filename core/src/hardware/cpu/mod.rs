//! The CPU is the main executor of any ROM's code, and will also keep
//! track of the cycles the CPU has performed so far.

use crate::emulator::*;
use crate::hardware::cpu::execute::{InstructionAddress, JumpModifier, WrapperEnum};

use crate::hardware::cpu::traits::{SetU16, SetU8, ToU16, ToU8};

use crate::hardware::memory::*;
use crate::hardware::registers::Reg8::A;
use crate::hardware::registers::{Flags, Reg16, Registers};
use crate::io::interrupts::Interrupts;

use log::*;
use std::fmt::*;
use crate::hardware::cpu::instructions::get_assembly_from_opcode;

#[cfg(test)]
mod tests;

mod alu;
mod execute;
mod fetch;
mod instructions;
mod traits;

#[derive(Debug)]
pub struct CPU<M: MemoryMapper> {
    pub cycles_performed: u128,
    pub ime: bool,
    pub halted: bool,
    pub mmu: M,
    opcode: u8,
    registers: Registers,
    delayed_ime: bool,
}

impl<M: MemoryMapper> CPU<M> {
    pub fn new(mmu: M) -> Self {
        let boot_rom_finished = mmu.boot_rom_finished();

        let mut result = CPU {
            opcode: 0,
            registers: Registers::new(),
            mmu,
            halted: false,
            cycles_performed: 0,
            ime: false,
            delayed_ime: false,
        };

        if boot_rom_finished {
            result.registers.pc = 0x100;
            // Set the registers to the state they would
            // have if we used the bootrom, missing MEM values
            result.registers.set_af(0x01B0);
            result.registers.set_bc(0x0013);
            result.registers.set_de(0x00D8);
            result.registers.set_hl(0x014D);
            result.registers.sp = 0xFFFE;
            // Initialise the DIV register to the value it would have had we run the bootrom.
            result.mmu.write_byte(0xFF04, 0xAB);
        }

        result
    }

    /// Fetches the next instruction and executes it as well.
    pub fn step_cycle(&mut self) {
        if self.halted {
            self.add_cycles();
            return;
        }
        // For the EI instruction, kinda dirty atm.
        // TODO:Think of a better architecture to accommodate delayed instructions
        //  (as DI will need it for GBC)
        if self.delayed_ime {
            self.ime = true;
            self.delayed_ime = false;
        }

        self.opcode = self.get_instr_u8();

        // trace!(
        //     "Executing opcode: {:04X} - registers: {}",
        //     self.opcode,
        //     self.registers,
        // );
        // let ie = self.mmu.read_byte(INTERRUPTS_ENABLE);
        // let if_flag = self.mmu.read_byte(INTERRUPTS_FLAG);
        // // trace!(
        // //     "Executing opcode: {:04X} - name: {:<22}\n----------- registers: {} - IE: {:02X} - IF: {:02X} - ime: {}",
        // //     self.opcode,
        // //     get_assembly_from_opcode(self.opcode),
        // //     self.registers,
        // //     ie,
        // //     if_flag,
        // //     self.ime
        // // );

        self.execute(self.opcode);
    }

    /// The routine to be used whenever any kind of `interrupt` is called.
    /// This will reset the `ime` flag and jump to the proper interrupt address.
    pub fn interrupts_routine(&mut self, interrupt: Interrupts) {
        use Interrupts::*;
        debug!("Interrupt called! {:?}", interrupt);
        // Two wait cycles
        self.add_cycles();
        self.add_cycles();

        self.ime = false;
        self.halted = false;
        self.push_helper(self.registers.pc);

        self.registers.pc = match interrupt {
            VBLANK => 0x0040,
            LcdStat => 0x0048,
            TIMER => 0x0050,
            SERIAL => 0x0058,
            JOYPAD => 0x0060,
        };
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
        T: Debug,
        Self: SetU8<T>,
        Self: ToU8<U>,
    {
        let source_value = self.read_u8_value(source);

        trace!("LD {:?} 0x{:02X}", destination, source_value);

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
        // Special increment as this function doesn't do any direct memory access.
        self.add_cycles();
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
    fn add16(&mut self, target: Reg16) {
        let old_value = self.read_u16_value(target);
        let (result, overflowed) = old_value.overflowing_add(self.registers.hl());
        self.registers.set_n(false);
        self.registers.set_cf(overflowed);
        self.registers
            .set_h((old_value & 0x0FFF) + (self.registers.hl() & 0x0FFF) > 0x0FFF);

        self.registers.set_hl(result);
        // Special increment as this function doesn't do any direct memory access.
        self.add_cycles();
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
        self.registers.set_h(old_value & 0xF == 0);

        self.set_u8_value(target, new_value);
    }

    /// `rr = rr-1      ;rr may be BC,DE,HL,SP`
    ///
    /// Flags: `----`
    fn decrement16(&mut self, target: Reg16) {
        let new_value = self.read_u16_value(target).wrapping_sub(1);

        self.set_u16_value(target, new_value);
        // Special increment as this function doesn't do any direct memory access.
        self.add_cycles();
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
        let offset = self.get_instr_u8() as i8;
        if self.matches_jmp_condition(condition) {
            // No idea why this works, but wrapping_sub/add depending on negative/positive value
            // always caused an addition even when casting to u16.
            self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
            self.add_cycles();
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
        log::info!("Entering halt");
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
        let value = self.read_u8_value(target);
        let carry_flag = self.registers.cf() as u8;
        let new_value = self.registers.a.wrapping_add(value).wrapping_add(carry_flag);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(false);
        self.registers.set_h((self.registers.a & 0xF) + (value & 0xF) + carry_flag > 0xF);
        self.registers.set_cf((self.registers.a as u16) + (value as u16) + carry_flag as u16 > 0xFF);


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
        let new_value = self.registers.a.wrapping_sub(value);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(true);
        self.registers.set_h((self.registers.a & 0xF).wrapping_sub(value & 0xF) & (0x10) != 0);
        self.registers.set_cf(value > self.registers.a);

        self.registers.a = new_value;
    }

    /// Subtract the value in `target` and the carry flag from A.
    ///
    /// Flags: `Z1HC`
    fn sbc<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
    {
        let value = self.read_u8_value(target);
        let carry_flag = self.registers.cf() as u8;
        let new_value = self.registers.a.wrapping_sub(value).wrapping_sub(carry_flag);

        self.registers.set_zf(new_value == 0);
        self.registers.set_n(true);
        self.registers.set_h((self.registers.a & 0xF).wrapping_sub(value & 0xF).wrapping_sub(carry_flag) & (0x10) != 0);
        self.registers.set_cf((value as u16 + carry_flag as u16) > self.registers.a as u16);

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
        let new_value = self.registers.a.wrapping_sub(value);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(true);
        self.registers.set_cf(value > self.registers.a);
        self.registers.set_h((self.registers.a & 0xF).wrapping_sub(value & 0xF) & (0x10) != 0);
    }

    /// Return from subroutine.
    /// This is basically a POP PC (if such an instruction existed).
    fn ret(&mut self, target: JumpModifier) {
        self.add_cycles();
        if self.matches_jmp_condition(target) {
            self.registers.pc = self.read_short_cycle(self.registers.sp);
            self.registers.sp = self.registers.sp.wrapping_add(2);
            self.add_cycles();
        }
    }

    /// Pop register `target` from the stack.
    ///
    /// Flags: `----`
    fn pop(&mut self, target: Reg16) {
        let sp_target = self.read_short_cycle(self.registers.sp);
        self.set_u16_value(target, sp_target);
        self.registers.sp = self.registers.sp.wrapping_add(2);
    }

    /// `jump to nn, PC=nn` OR `jump to HL, PC=HL` OR `conditional jump if nz,z,nc,c`
    /// Sets the `PC` to the relevant value based on the JumpCondition.
    fn jump(&mut self, condition: JumpModifier) {
        let value = self.get_instr_u16();

        if self.matches_jmp_condition(condition) {
            self.registers.pc = if let JumpModifier::HL = condition {
                //TODO: Consider moving to separate function to clean up enum.
                self.registers.hl()
            } else {
                value
            };

            self.add_cycles();
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
        let address = self.get_instr_u16();
        if self.matches_jmp_condition(target) {
            self.push_helper(self.registers.pc);
            self.registers.pc = address;
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
        self.write_short_cycle(self.registers.sp, value);
        self.add_cycles();
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
        self.ime = true;
        self.registers.pc = self.read_short_cycle(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(2);
        self.add_cycles();
    }

    /// `ADD SP,e8`
    /// Add the signed value e8 to SP.
    ///
    /// Flags: `00HC`
    fn add_sp(&mut self) {
        let value = self.get_instr_u8() as i8 as u16;
        let new_value = self.registers.sp.wrapping_add(value);

        self.registers.set_zf(false);
        self.registers.set_n(false);
        self.registers.set_h((self.registers.sp & 0xF) + (value & 0xF) > 0xF);
        self.registers.set_cf((self.registers.sp & 0xFF) + (value & 0xFF) > 0xFF);

        self.registers.sp = new_value;

        self.add_cycles();
        self.add_cycles();
    }

    /// `DI`
    /// Disable Interrupts by clearing the IME flag.
    ///
    /// Flags: `----`
    fn di(&mut self) {
        self.ime = false;
        // Never, ever, ever, get it in your head to add this again. 2 Days of debugging ._.
        //self.mmu.write_byte(INTERRUPTS_ENABLE, 0x0);
    }

    /// `LD HL,SP+i8`
    /// Load the value of `SP + i8` into the register `HL`.
    ///
    /// Flags: `00HC`
    fn load_sp_i(&mut self) {
        let value = self.get_instr_u8() as i8 as u16;
        let new_value = self.registers.sp.wrapping_add(value);

        self.registers.set_hl(new_value);
        self.registers.set_zf(false);
        self.registers.set_n(false);
        self.registers.set_h((self.registers.sp & 0xF) + (value & 0xF) > 0xF);
        // Test if overflow on 7th bit.
        self.registers.set_cf((self.registers.sp & 0xFF) + (value & 0xFF) > 0xFF);

        self.add_cycles();
    }

    /// `LD SP, HL`
    /// Load the value of `HL` into `SP`
    ///
    /// Flags: `----`
    fn load_sp_hl(&mut self) {
        self.registers.sp = self.registers.hl();
        self.add_cycles();
    }

    /// `EI`
    /// Enable Interrupts by setting the IME flag.
    /// The flag is only set after the instruction following EI.
    fn ei(&mut self) {
        //TODO: Actually do this properly
        self.delayed_ime = true;
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
    fn bit<T: Copy + Debug>(&mut self, bit: u8, target: T)
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
