use crate::hardware::cpu::execute::InstructionAddress;
use crate::hardware::cpu::CPU;
use crate::hardware::memory::MemoryMapper;
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

impl<T: MemoryMapper> ToU8<Reg8> for CPU<T> {
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

impl<T: MemoryMapper> SetU8<Reg8> for CPU<T> {
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

impl<T: MemoryMapper> ToU8<InstructionAddress> for CPU<T> {
    fn read_u8_value(&mut self, target: InstructionAddress) -> u8 {
        use crate::hardware::memory::IO_START;
        use InstructionAddress::*;

        match target {
            BCI => self.read_byte_cycle(self.registers.bc()),
            DEI => self.read_byte_cycle(self.registers.de()),
            HLI => self.read_byte_cycle(self.registers.hl()),
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
                self.read_byte_cycle(address)
            }
            IoDirect => {
                let address = self.get_instr_u8() as u16;
                log::trace!("IoDirect read from address: 0x{:04X}", IO_START + address);
                self.read_byte_cycle(IO_START + address)
            }
            IoC => self.read_byte_cycle(IO_START + self.registers.c as u16),
        }
    }
}

impl<T: MemoryMapper> SetU8<InstructionAddress> for CPU<T> {
    fn set_u8_value(&mut self, target: InstructionAddress, value: u8) {
        use crate::hardware::memory::IO_START;
        use InstructionAddress::*;

        match target {
            BCI => self.write_byte_cycle(self.registers.bc(), value),
            DEI => self.write_byte_cycle(self.registers.de(), value),
            HLI => self.write_byte_cycle(self.registers.hl(), value),
            HLIP => {
                self.set_u8_value(HLI, value);
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
            }
            HLIN => {
                self.set_u8_value(HLI, value);
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
            }
            DIRECT | DirectMem => {
                let address = self.get_instr_u16();
                log::trace!(
                    "Direct memory write to address: 0x{:04X} with value: 0x{:02X}",
                    address,
                    value
                );
                self.write_byte_cycle(address, value);
            }
            IoDirect => {
                let addition = self.get_instr_u8() as u16;
                log::trace!(
                    "IoDirect write to address: 0x{:04X} with value: 0x{:02X}",
                    IO_START + addition,
                    value
                );
                self.write_byte_cycle(IO_START + addition, value);
            }
            IoC => self.write_byte_cycle(IO_START + self.registers.c as u16, value),
        }
    }
}

impl<T: MemoryMapper> ToU8<u8> for CPU<T> {
    fn read_u8_value(&mut self, target: u8) -> u8 {
        target
    }
}

impl<T: MemoryMapper> ToU16<Reg16> for CPU<T> {
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

impl<T: MemoryMapper> SetU16<Reg16> for CPU<T> {
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

impl<T: MemoryMapper> ToU16<InstructionAddress> for CPU<T> {
    fn read_u16_value(&mut self, target: InstructionAddress) -> u16 {
        use InstructionAddress::*;

        match target {
            DIRECT => self.get_instr_u16(),
            _ => unimplemented!(),
        }
    }
}

impl<T: MemoryMapper> SetU16<InstructionAddress> for CPU<T> {
    fn set_u16_value(&mut self, target: InstructionAddress, value: u16) {
        use InstructionAddress::*;

        match target {
            DIRECT | DirectMem => {
                let address = self.get_instr_u16();
                self.write_short_cycle(address, value);
            }
            _ => unimplemented!(),
        }
    }
}
