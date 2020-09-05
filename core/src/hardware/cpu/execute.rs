use crate::hardware::cpu::CPU;
use crate::hardware::memory::MemoryMapper;
use crate::hardware::registers::Reg8;

impl<M: MemoryMapper> CPU<M> {

    pub fn execute(&mut self, opcode: u8) {
        use crate::hardware::registers::Reg16::*;
        use crate::hardware::registers::Reg8::*;
        match opcode {
            0x00 => self.nop(),
            0x01 => self.load_16bit(BC, InstructionAddress::DIRECT),
            0x02 => self.load_8bit(InstructionAddress::BCI, A),
            0x03 => self.increment16(BC),
            0x04 => self.increment(B),
            0x05 => self.decrement(B),
            0x06 => self.load_8bit(B, InstructionAddress::DIRECT),
            0x07 => self.rlca(),
            0x08 => self.load_16bit(InstructionAddress::DirectMem, SP),
            0x09 => self.add16(BC),
            0x0A => self.load_8bit(A, InstructionAddress::BCI),
            0x0B => self.decrement16(BC),
            0x0C => self.increment(C),
            0x0D => self.decrement(C),
            0x0E => self.load_8bit(C, InstructionAddress::DIRECT),
            0x0F => self.rrca(),
            0x10 => self.stop(),
            0x11 => self.load_16bit(DE, InstructionAddress::DIRECT),
            0x12 => self.load_8bit(InstructionAddress::DEI, A),
            0x13 => self.increment16(DE),
            0x14 => self.increment(D),
            0x15 => self.decrement(D),
            0x16 => self.load_8bit(D, InstructionAddress::DIRECT),
            0x17 => self.rla(),
            0x18 => self.relative_jump(JumpModifier::Always),
            0x19 => self.add16(DE),
            0x1A => self.load_8bit(A, InstructionAddress::DEI),
            0x1B => self.decrement16(DE),
            0x1C => self.increment(E),
            0x1D => self.decrement(E),
            0x1E => self.load_8bit(E, InstructionAddress::DIRECT),
            0x1F => self.rra(),
            0x20 => self.relative_jump(JumpModifier::NotZero),
            0x21 => self.load_16bit(HL, InstructionAddress::DIRECT),
            0x22 => self.load_8bit(InstructionAddress::HLIP, A),
            0x23 => self.increment16(HL),
            0x24 => self.increment(H),
            0x25 => self.decrement(H),
            0x26 => self.load_8bit(H, InstructionAddress::DIRECT),
            0x27 => self.daa(),
            0x28 => self.relative_jump(JumpModifier::Zero),
            0x29 => self.add16(HL),
            0x2A => self.load_8bit(A, InstructionAddress::HLIP),
            0x2B => self.decrement16(HL),
            0x2C => self.increment(L),
            0x2D => self.decrement(L),
            0x2E => self.load_8bit(L, InstructionAddress::DIRECT),
            0x2F => self.cpl(),
            0x30 => self.relative_jump(JumpModifier::NotCarry),
            0x31 => self.load_16bit(SP, InstructionAddress::DIRECT),
            0x32 => self.load_8bit(InstructionAddress::HLIN, A),
            0x33 => self.increment16(SP),
            0x34 => self.increment(InstructionAddress::HLI),
            0x35 => self.decrement(InstructionAddress::HLI),
            0x36 => self.load_8bit(InstructionAddress::HLI, InstructionAddress::DIRECT),
            0x37 => self.scf(),
            0x38 => self.relative_jump(JumpModifier::Carry),
            0x39 => self.add16(SP),
            0x3A => self.load_8bit(A, InstructionAddress::HLIN),
            0x3B => self.decrement16(SP),
            0x3C => self.increment(A),
            0x3D => self.decrement(A),
            0x3E => self.load_8bit(A, InstructionAddress::DIRECT),
            0x3F => self.ccf(),
            0x40..=0x75 => self.load_8bit(vertical_decode(opcode), horizontal_decode(opcode)),
            0x76 => self.halt(),
            0x77..=0x7F => self.load_8bit(vertical_decode(opcode), horizontal_decode(opcode)),
            0x80..=0x87 => self.add(horizontal_decode(opcode)),
            0x88..=0x8F => self.adc(horizontal_decode(opcode)),
            0x90..=0x97 => self.sub(horizontal_decode(opcode)),
            0x98..=0x9F => self.sbc(horizontal_decode(opcode)),
            0xA0..=0xA7 => self.and(horizontal_decode(opcode)),
            0xA8..=0xAF => self.xor(horizontal_decode(opcode)),
            0xB0..=0xB7 => self.or(horizontal_decode(opcode)),
            0xB8..=0xBF => self.compare(horizontal_decode(opcode)),
            0xC0 => self.ret(JumpModifier::NotZero),
            0xC1 => self.pop(BC),
            0xC2 => self.jump(JumpModifier::NotZero),
            0xC3 => self.jump(JumpModifier::Always),
            0xC4 => self.call(JumpModifier::NotZero),
            0xC5 => self.push(BC),
            0xC6 => self.add(InstructionAddress::DIRECT),
            0xC7 => self.rst(0x0),
            0xC8 => self.ret(JumpModifier::Zero),
            0xC9 => self.ret(JumpModifier::Always),
            0xCA => self.jump(JumpModifier::Zero),
            0xCB => self.cb_prefix_call(),
            0xCC => self.call(JumpModifier::Zero),
            0xCD => self.call(JumpModifier::Always),
            0xCE => self.adc(InstructionAddress::DIRECT),
            0xCF => self.rst(0x8),
            0xD0 => self.ret(JumpModifier::NotCarry),
            0xD1 => self.pop(DE),
            0xD2 => self.jump(JumpModifier::NotCarry),
            0xD3 => self.unknown(),
            0xD4 => self.call(JumpModifier::NotCarry),
            0xD5 => self.push(DE),
            0xD6 => self.sub(InstructionAddress::DIRECT),
            0xD7 => self.rst(0x10),
            0xD8 => self.ret(JumpModifier::Carry),
            0xD9 => self.reti(),
            0xDA => self.jump(JumpModifier::Carry),
            0xDB => self.unknown(),
            0xDC => self.call(JumpModifier::Carry),
            0xDD => self.unknown(),
            0xDE => self.sbc(InstructionAddress::DIRECT),
            0xDF => self.rst(0x18),
            0xE0 => self.load_8bit(InstructionAddress::IoDirect, A),
            0xE1 => self.pop(HL),
            0xE2 => self.load_8bit(InstructionAddress::IoC, A),
            0xE3 | 0xE4 => self.unknown(),
            0xE5 => self.push(HL),
            0xE6 => self.and(InstructionAddress::DIRECT),
            0xE7 => self.rst(0x20),
            0xE8 => self.add_sp(),
            0xE9 => self.jump(JumpModifier::HL),
            0xEA => self.load_8bit(InstructionAddress::DirectMem, A),
            0xEB..=0xED => self.unknown(),
            0xEE => self.xor(InstructionAddress::DIRECT),
            0xEF => self.rst(0x28),
            0xF0 => self.load_8bit(A, InstructionAddress::IoDirect),
            0xF1 => self.pop(AF),
            0xF2 => self.load_8bit(A, InstructionAddress::IoC),
            0xF3 => self.di(),
            0xF4 => self.unknown(),
            0xF5 => self.push(AF),
            0xF6 => self.or(InstructionAddress::DIRECT),
            0xF7 => self.rst(0x30),
            0xF8 => self.load_sp_i(),
            0xF9 => self.load_sp_hl(),
            0xFA => self.load_8bit(A, InstructionAddress::DirectMem),
            0xFB => self.ei(),
            0xFC | 0xFD => self.unknown(),
            0xFE => self.compare(InstructionAddress::DIRECT),
            0xFF => self.rst(0x38),
            _ => panic!("Unknown instruction code encountered: {:X}", opcode),
        }
    }

    pub fn execute_prefix(&mut self, opcode: u8) {
        match opcode {
            0x00..=0x07 => self.rlc(horizontal_decode(opcode)),
            0x08..=0x0F => self.rrc(horizontal_decode(opcode)),
            0x10..=0x17 => self.rl(horizontal_decode(opcode)),
            0x18..=0x1F => self.rr(horizontal_decode(opcode)),
            0x20..=0x27 => self.sla(horizontal_decode(opcode)),
            0x28..=0x2F => self.sra(horizontal_decode(opcode)),
            0x30..=0x37 => self.swap(horizontal_decode(opcode)),
            0x38..=0x3F => self.srl(horizontal_decode(opcode)),
            0x40..=0x7F => self.bit(decode_prefixed_bit(opcode), horizontal_decode(opcode)),
            0x80..=0xBF => self.res(decode_prefixed_bit(opcode), horizontal_decode(opcode)),
            0xC0..=0xFF => self.set(decode_prefixed_bit(opcode), horizontal_decode(opcode)),
            _ => panic!("Unknown prefix instruction code encountered: {:X}", opcode),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum InstructionAddress {
    BCI,
    DEI,
    HLI,
    HLIP,
    HLIN,
    DIRECT,
    DirectMem,
    IoDirect,
    IoC,
}

/// Only exists to make the instruction decoding a bit less of a hassle
#[derive(Debug, Copy, Clone)]
pub enum WrapperEnum {
    Reg8(Reg8),
    InstructionAddress(InstructionAddress),
}

pub fn decode_prefixed_bit(opcode: u8) -> u8 {
    let relevant_nibble = ((opcode & 0xF0) >> 4) % 0x4;
    let lower_nibble = opcode & 0x0F;
    match relevant_nibble {
        0x0 if lower_nibble > 7 => 1,
        0x0 => 0,
        0x1 if lower_nibble > 7 => 3,
        0x1 => 2,
        0x2 if lower_nibble > 7 => 5,
        0x2 => 4,
        0x3 if lower_nibble > 7 => 7,
        0x3 => 6,
        _ => panic!(
            "Encountered out of scope bit for relevant nib: {} and lower nib {}",
            relevant_nibble, lower_nibble
        ),
    }
}

pub fn horizontal_decode(opcode: u8) -> WrapperEnum {
    let relevant_nibble = (opcode & 0x0F) % 0x8;
    match relevant_nibble {
        0x0 => WrapperEnum::Reg8(Reg8::B),
        0x1 => WrapperEnum::Reg8(Reg8::C),
        0x2 => WrapperEnum::Reg8(Reg8::D),
        0x3 => WrapperEnum::Reg8(Reg8::E),
        0x4 => WrapperEnum::Reg8(Reg8::H),
        0x5 => WrapperEnum::Reg8(Reg8::L),
        0x6 => WrapperEnum::InstructionAddress(InstructionAddress::HLI),
        0x7 => WrapperEnum::Reg8(Reg8::A),
        // This should never be called, unless maths has broken down.
        _ => panic!("Invalid Nibble found: {:X}", relevant_nibble),
    }
}

pub fn vertical_decode(opcode: u8) -> WrapperEnum {
    let relevant_nibble = (opcode & 0xF0) >> 4;
    let lower_nibble = opcode & 0x0F;
    match relevant_nibble {
        0x4 if lower_nibble < 0x8 => WrapperEnum::Reg8(Reg8::B),
        0x4 if lower_nibble >= 0x8 => WrapperEnum::Reg8(Reg8::C),
        0x5 if lower_nibble < 0x8 => WrapperEnum::Reg8(Reg8::D),
        0x5 if lower_nibble >= 0x8 => WrapperEnum::Reg8(Reg8::E),
        0x6 if lower_nibble < 0x8 => WrapperEnum::Reg8(Reg8::H),
        0x6 if lower_nibble >= 0x8 => WrapperEnum::Reg8(Reg8::L),
        0x7 if lower_nibble < 0x8 => WrapperEnum::InstructionAddress(InstructionAddress::HLI),
        0x7 if lower_nibble >= 0x8 => WrapperEnum::Reg8(Reg8::A),
        _ => panic!("Invalid Nibble found: {:X}", relevant_nibble),
    }
}

#[derive(Debug, Copy, Clone)]
pub enum JumpModifier {
    NotZero,
    Zero,
    NotCarry,
    Carry,
    Always,
    HL,
}
