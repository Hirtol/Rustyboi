use crate::hardware::cpu::execute::{InstructionAddress, WrapperEnum};
use crate::hardware::cpu::CPU;
use crate::hardware::registers::{Reg16, Reg8};

/// This trait should be used where we might pass either a direct
/// registry address, or a combined registry which points to a memory address.
///
/// In hindsight, could've probably also just used `Into`
pub trait ToU8<T: Copy> {
    /// Calling this function should automatically resolve the address directly to
    /// a value, regardless if it was a registry address or a pointer to memory.
    fn read_u8_value(&mut self, target: T) -> u8;
}

pub trait SetU8<T: Copy> {
    fn set_u8_value(&mut self, target: T, value: u8);
}

pub trait ToU16<T: Copy> {
    fn read_u16_value(&mut self, target: T) -> u16;
}

pub trait SetU16<T: Copy> {
    fn set_u16_value(&mut self, target: T, value: u16);
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
        use crate::hardware::memory::IO_START;
        use InstructionAddress::*;

        match target {
            BCI => self.mmu.borrow().read_byte(self.registers.bc()),
            DEI => self.mmu.borrow().read_byte(self.registers.de()),
            HLI => self.mmu.borrow().read_byte(self.registers.hl()),
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
                self.mmu.borrow().read_byte(address)
            }
            IoDirect => {
                let address = self.get_instr_u8() as u16;
                self.mmu.borrow().read_byte(IO_START + address)
            }
            IoC => self.mmu.borrow().read_byte(IO_START + self.registers.c as u16),
        }
    }
}

impl SetU8<InstructionAddress> for CPU {
    fn set_u8_value(&mut self, target: InstructionAddress, value: u8) {
        use crate::hardware::memory::IO_START;
        use InstructionAddress::*;

        match target {
            BCI => self.mmu.borrow_mut().set_byte(self.registers.bc(), value),
            DEI => self.mmu.borrow_mut().set_byte(self.registers.de(), value),
            HLI => self.mmu.borrow_mut().set_byte(self.registers.hl(), value),
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
                self.mmu.borrow_mut().set_byte(address, value)
            }
            DirectMem => {
                let address = self.get_instr_u16();
                self.mmu.borrow_mut().set_byte(address, value)
            }
            IoDirect => {
                let addition = self.get_instr_u8() as u16;
                self.mmu.borrow_mut().set_byte(IO_START + addition, value);
            }
            IoC => self.mmu.borrow_mut().set_byte(IO_START + self.registers.c as u16, value),
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
    fn read_u16_value(&mut self, target: Reg16) -> u16 {
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
    fn read_u16_value(&mut self, target: InstructionAddress) -> u16 {
        use crate::hardware::memory::IO_START;
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
        use crate::hardware::memory::IO_START;
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
                self.mmu.borrow_mut().set_short(address, value);
                // self.mmu.borrow().set_byte(address, (value & 0x0F) as u8);
                // self.mmu.borrow().set_byte(address.wrapping_add(1), (value & 0xF0 >> 8) as u8);
            }
            DirectMem => {
                let address = self.get_instr_u16();
                self.mmu.borrow_mut().set_short(address, value);
            }
            IoDirect => unimplemented!(),
            IoC => unimplemented!(),
        }
    }
}
