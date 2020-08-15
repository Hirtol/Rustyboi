use crate::hardware::cpu::CPU;
use crate::hardware::registers::Reg8;

impl CPU {
    pub fn execute(&mut self, opcode: u8) {
        use crate::hardware::registers::Reg16::*;
        match opcode {
            0x00 => return,
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
            0xC8 => self.ret(JumpModifier::Zero),
            0xC9 => self.ret(JumpModifier::Always),
            0xCA => self.jump(JumpModifier::Zero),
            0xCC => self.call(JumpModifier::Zero),
            0xCD => self.call(JumpModifier::Always),
            0xD0 => self.ret(JumpModifier::NotCarry),
            0xD1 => self.pop(DE),
            0xD2 => self.jump(JumpModifier::NotCarry),
            0xD4 => self.call(JumpModifier::NotCarry),
            0xD5 => self.push(DE),
            0xD8 => self.ret(JumpModifier::Carry),
            0xDA => self.jump(JumpModifier::Carry),
            0xDC => self.call(JumpModifier::Carry),
            0xE1 => self.pop(HL),
            0xE5 => self.push(HL),
            0xE9 => self.jump(JumpModifier::HL),
            0xF1 => self.pop(AF),
            0xF5 => self.push(AF),
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
    HLID,
    ADDR,
}

#[derive(Debug, Copy, Clone)]
pub enum WrapperEnum {
    Reg8(Reg8),
    InstructionAddress(InstructionAddress),
}

fn decode_prefixed_bit(opcode: u8) -> u8 {
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

fn horizontal_decode(opcode: u8) -> WrapperEnum {
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

fn vertical_decode(opcode: u8) -> WrapperEnum {
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
