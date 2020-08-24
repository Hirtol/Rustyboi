//! Deprecated as it turned out to be more trouble than it was worth
//! for the small bit of extra clarity that an enum would provide
//! Keep around in case we want to turn back.

use crate::hardware::cpu::execute::{
    horizontal_decode, vertical_decode, InstructionAddress, JumpModifier,
};
use crate::hardware::registers::Reg16;
use crate::hardware::registers::Reg16::*;
use crate::hardware::registers::Reg8::*;

pub fn get_assembly_from_opcode(opcode: u8) -> String {
    match opcode {
        0x00 => format!("nop"),
        0x01 => format!("load_16bit {:?} {:?}", BC, InstructionAddress::DIRECT),
        0x02 => format!("load_8bit {:?} {:?}", InstructionAddress::BCI, A),
        0x03 => format!("increment16 {:?}", BC),
        0x04 => format!("increment {:?}", B),
        0x05 => format!("decrement {:?}", B),
        0x06 => format!("load_8bit {:?} {:?}", B, InstructionAddress::DIRECT),
        0x07 => format!("rlca"),
        0x08 => format!("load_16bit {:?} {:?}", InstructionAddress::DirectMem, SP),
        0x09 => format!("add_16bit {:?} ", BC),
        0x0A => format!("load_8bit {:?} {:?}", A, InstructionAddress::BCI),
        0x0B => format!("decrement16 {:?}", BC),
        0x0C => format!("increment {:?}", C),
        0x0D => format!("decrement {:?}", C),
        0x0E => format!("load_8bit {:?} {:?}", C, InstructionAddress::DIRECT),
        0x0F => format!("rrca"),
        0x10 => format!("stop"),
        0x11 => format!("load_16bit {:?} {:?}", DE, InstructionAddress::DIRECT),
        0x12 => format!("load_8bit {:?} {:?}", InstructionAddress::DEI, A),
        0x13 => format!("increment16 {:?}", DE),
        0x14 => format!("increment {:?}", D),
        0x15 => format!("decrement {:?}", D),
        0x16 => format!("load_8bit {:?} {:?}", D, InstructionAddress::DIRECT),
        0x17 => format!("rla"),
        0x18 => format!("relative_jump {:?}", JumpModifier::Always),
        0x19 => format!("add_16bit {:?}", DE),
        0x1A => format!("load_8bit {:?} {:?}", A, InstructionAddress::DEI),
        0x1B => format!("decrement16 {:?}", DE),
        0x1C => format!("increment {:?}", E),
        0x1D => format!("decrement {:?}", E),
        0x1E => format!("load_8bit {:?} {:?}", E, InstructionAddress::DIRECT),
        0x1F => format!("rra"),
        0x20 => format!("relative_jump {:?}", JumpModifier::NotZero),
        0x21 => format!("load_16bit {:?} {:?}", HL, InstructionAddress::DIRECT),
        0x22 => format!("load_8bit {:?} {:?}", InstructionAddress::HLIP, A),
        0x23 => format!("increment16 {:?}", HL),
        0x24 => format!("increment {:?}", H),
        0x25 => format!("decrement {:?}", H),
        0x26 => format!("load_8bit {:?} {:?}", H, InstructionAddress::DIRECT),
        0x27 => format!("daa"),
        0x28 => format!("relative_jump {:?}", JumpModifier::Zero),
        0x29 => format!("add_16bit {:?}", HL),
        0x2A => format!("load_8bit {:?} {:?}", A, InstructionAddress::HLIP),
        0x2B => format!("decrement16 {:?}", HL),
        0x2C => format!("increment {:?}", L),
        0x2D => format!("decrement {:?}", L),
        0x2E => format!("load_8bit {:?} {:?}", L, InstructionAddress::DIRECT),
        0x2F => format!("cpl"),
        0x30 => format!("relative_jump {:?}", JumpModifier::NotCarry),
        0x31 => format!("load_16bit {:?} {:?}", SP, InstructionAddress::DIRECT),
        0x32 => format!("load_8bit {:?} {:?}", InstructionAddress::HLIN, A),
        0x33 => format!("increment16 {:?}", SP),
        0x34 => format!("increment {:?}", InstructionAddress::HLI),
        0x35 => format!("decrement {:?}", InstructionAddress::HLI),
        0x36 => format!(
            "load_8bit {:?} {:?}",
            InstructionAddress::HLI,
            InstructionAddress::DIRECT
        ),
        0x37 => format!("scf"),
        0x38 => format!("relative_jump {:?}", JumpModifier::Carry),
        0x39 => format!("add_16bit {:?}", SP),
        0x3A => format!("load_8bit {:?} {:?}", A, InstructionAddress::HLIN),
        0x3B => format!("decrement16 {:?}", SP),
        0x3C => format!("increment {:?}", A),
        0x3D => format!("decrement {:?}", A),
        0x3E => format!("load_8bit {:?} {:?}", A, InstructionAddress::DIRECT),
        0x3F => format!("ccf"),
        0x40..=0x75 => format!(
            "load_8bit {:?} {:?}",
            vertical_decode(opcode),
            horizontal_decode(opcode)
        ),
        0x76 => format!("halt"),
        0x77..=0x7F => format!(
            "load_8bit {:?} {:?}",
            vertical_decode(opcode),
            horizontal_decode(opcode)
        ),
        0x80..=0x87 => format!("add {:?}", horizontal_decode(opcode)),
        0x88..=0x8F => format!("adc {:?}", horizontal_decode(opcode)),
        0x90..=0x97 => format!("sub {:?}", horizontal_decode(opcode)),
        0x98..=0x9F => format!("sbc {:?}", horizontal_decode(opcode)),
        0xA0..=0xA7 => format!("and {:?}", horizontal_decode(opcode)),
        0xA8..=0xAF => format!("xor {:?}", horizontal_decode(opcode)),
        0xB0..=0xB7 => format!("or {:?}", horizontal_decode(opcode)),
        0xB8..=0xBF => format!("compare {:?}", horizontal_decode(opcode)),
        0xC0 => format!("ret {:?}", JumpModifier::NotZero),
        0xC1 => format!("pop {:?}", BC),
        0xC2 => format!("jump {:?}", JumpModifier::NotZero),
        0xC3 => format!("jump {:?}", JumpModifier::Always),
        0xC4 => format!("call {:?}", JumpModifier::NotZero),
        0xC5 => format!("push {:?}", BC),
        0xC6 => format!("add {:?}", InstructionAddress::DIRECT),
        0xC7 => format!("rst {:?}", 0x0),
        0xC8 => format!("ret {:?}", JumpModifier::Zero),
        0xC9 => format!("ret {:?}", JumpModifier::Always),
        0xCA => format!("jump {:?}", JumpModifier::Zero),
        0xCB => panic!("Regular executor function should not be passed the CB prefix!"),
        0xCC => format!("call {:?}", JumpModifier::Zero),
        0xCD => format!("call {:?}", JumpModifier::Always),
        0xCE => format!("adc {:?}", InstructionAddress::DIRECT),
        0xCF => format!("rst {:?}", 0x8),
        0xD0 => format!("ret {:?}", JumpModifier::NotCarry),
        0xD1 => format!("pop {:?}", DE),
        0xD2 => format!("jump {:?}", JumpModifier::NotCarry),
        0xD3 => format!("unknown"),
        0xD4 => format!("call {:?}", JumpModifier::NotCarry),
        0xD5 => format!("push {:?}", DE),
        0xD6 => format!("sub {:?}", InstructionAddress::DIRECT),
        0xD7 => format!("rst {:?}", 0x10),
        0xD8 => format!("ret {:?}", JumpModifier::Carry),
        0xD9 => format!("reti"),
        0xDA => format!("jump {:?}", JumpModifier::Carry),
        0xDB => format!("unknown"),
        0xDC => format!("call {:?}", JumpModifier::Carry),
        0xDD => format!("unknown"),
        0xDE => format!("sbc {:?}", InstructionAddress::DIRECT),
        0xDF => format!("rst {:?}", 0x18),
        0xE0 => format!("load_8bit {:?} {:?}", InstructionAddress::IoDirect, A),
        0xE1 => format!("pop {:?}", HL),
        0xE2 => format!("load_8bit {:?} {:?}", InstructionAddress::IoC, A),
        0xE3 | 0xE4 => format!("unknown"),
        0xE5 => format!("push {:?}", HL),
        0xE6 => format!("and {:?}", InstructionAddress::DIRECT),
        0xE7 => format!("rst {:?}", 0x20),
        0xE8 => format!("add SP i8"),
        0xE9 => format!("jump {:?}", JumpModifier::HL),
        0xEA => format!("load_8bit {:?} {:?}", InstructionAddress::DirectMem, A),
        0xEB..=0xED => format!("unknown"),
        0xEE => format!("xor {:?}", InstructionAddress::DIRECT),
        0xEF => format!("rst {:?}", 0x28),
        0xF0 => format!("load_8bit {:?} {:?}", A, InstructionAddress::IoDirect),
        0xF1 => format!("pop {:?}", AF),
        0xF2 => format!("load_8bit {:?} {:?}", A, InstructionAddress::IoC),
        0xF3 => format!("di"),
        0xF4 => format!("unknown"),
        0xF5 => format!("push {:?}", AF),
        0xF6 => format!("or {:?}", InstructionAddress::DIRECT),
        0xF7 => format!("rst {:?}", 0x30),
        0xF8 => format!("load HL SP+i8"),
        0xF9 => format!("load_16bit {:?} {:?}", SP, HL),
        0xFA => format!("load_8bit {:?} {:?}", A, InstructionAddress::DirectMem),
        0xFB => format!("ei"),
        0xFC | 0xFD => format!("unknown"),
        0xFE => format!("compare {:?}", InstructionAddress::DIRECT),
        0xFF => format!("rst {:?}", 0x38),
        _ => panic!("Unknown instruction code encountered: {:X}", opcode),
    }
}

#[derive(Debug)]
pub enum Instruction {
    NOP,
    LD(),
    INC,
    DEC,
    RLCA,
    RRCA,
    STOP,
    RLA,
    JR,
    RRA,
    DAA,
    CPL,
    SCF,
    CCF,
    HALT,
    ADD(RegistryTarget),
    ADC(RegistryTarget),
    SUB(RegistryTarget),
    SBC(RegistryTarget),
    AND(RegistryTarget),
    XOR(RegistryTarget),
    OR(RegistryTarget),
    CP(RegistryTarget),
    RET(JumpModifier),
    POP(Reg16),
    JP(JumpModifier),
    CALL(JumpModifier),
    PUSH(Reg16),
    RST,
    PREFIX,
    RETI,
    DI,
    EI,
    // Prefixed Instructions
    RLC(RegistryTarget),
    RRC(RegistryTarget),
    RL(RegistryTarget),
    RR(RegistryTarget),
    SLA(RegistryTarget),
    SRA(RegistryTarget),
    SWAP(RegistryTarget),
    SRL(RegistryTarget),
    BIT(u8, RegistryTarget),
    SET(u8, RegistryTarget),
    RES(u8, RegistryTarget),
}
//
// impl Instruction {
//     pub fn decode(opcode: u8) -> Self {
//         match opcode {
//             0x00 => Instruction::NOP,
//             0x40..=0x75 => Instruction::LD(LoadInfo::decode(opcode)),
//             0x76 => Instruction::HALT,
//             0x77..=0x7F => Instruction::LD(LoadInfo::decode(opcode)),
//             0x80..=0x87 => Instruction::ADD(RegistryTarget::decode(opcode)),
//             0x88..=0x8F => Instruction::ADC(RegistryTarget::decode(opcode)),
//             0x90..=0x97 => Instruction::SUB(RegistryTarget::decode(opcode)),
//             0x98..=0x9F => Instruction::SBC(RegistryTarget::decode(opcode)),
//             0xA0..=0xA7 => Instruction::AND(RegistryTarget::decode(opcode)),
//             0xA8..=0xAF => Instruction::XOR(RegistryTarget::decode(opcode)),
//             0xB0..=0xB7 => Instruction::OR(RegistryTarget::decode(opcode)),
//             0xB8..=0xBF => Instruction::CP(RegistryTarget::decode(opcode)),
//             0xC0 => Instruction::RET(JumpModifier::NotZero),
//             0xC1 => Instruction::POP(BC),
//             0xC2 => Instruction::JP(JumpModifier::NotZero),
//             0xC3 => Instruction::JP(JumpModifier::Always),
//             0xC4 => Instruction::CALL(JumpModifier::NotZero),
//             0xC5 => Instruction::PUSH(BC),
//             0xC8 => Instruction::RET(JumpModifier::Zero),
//             0xC9 => Instruction::RET(JumpModifier::Always),
//             0xCA => Instruction::JP(JumpModifier::Zero),
//             0xCC => Instruction::CALL(JumpModifier::Zero),
//             0xCD => Instruction::CALL(JumpModifier::Always),
//             0xD0 => Instruction::RET(JumpModifier::NotCarry),
//             0xD1 => Instruction::POP(DE),
//             0xD2 => Instruction::JP(JumpModifier::NotCarry),
//             0xD4 => Instruction::CALL(JumpModifier::NotCarry),
//             0xD5 => Instruction::PUSH(DE),
//             0xD8 => Instruction::RET(JumpModifier::Carry),
//             0xDA => Instruction::JP(JumpModifier::Carry),
//             0xDC => Instruction::CALL(JumpModifier::Carry),
//             0xE1 => Instruction::POP(HL),
//             0xE5 => Instruction::PUSH(HL),
//             0xE9 => Instruction::JP(JumpModifier::HL),
//             0xF1 => Instruction::POP(AF),
//             0xF5 => Instruction::PUSH(AF),
//             _ => panic!("Unknown instruction code encountered: {:X}", opcode),
//         }
//     }
//
//     pub fn decode_prefix(opcode: u8) -> Self {
//         match opcode {
//             0x00..=0x07 => Instruction::RLC(RegistryTarget::decode(opcode)),
//             0x08..=0x0F => Instruction::RRC(RegistryTarget::decode(opcode)),
//             0x10..=0x17 => Instruction::RL(RegistryTarget::decode(opcode)),
//             0x18..=0x1F => Instruction::RR(RegistryTarget::decode(opcode)),
//             0x20..=0x27 => Instruction::SLA(RegistryTarget::decode(opcode)),
//             0x28..=0x2F => Instruction::SRA(RegistryTarget::decode(opcode)),
//             0x30..=0x37 => Instruction::SWAP(RegistryTarget::decode(opcode)),
//             0x38..=0x3F => Instruction::SRL(RegistryTarget::decode(opcode)),
//             0x40..=0x7F => {
//                 Instruction::BIT(decode_prefixed_bit(opcode), RegistryTarget::decode(opcode))
//             }
//             0x80..=0xBF => {
//                 Instruction::RES(decode_prefixed_bit(opcode), RegistryTarget::decode(opcode))
//             }
//             0xC0..=0xFF => {
//                 Instruction::SET(decode_prefixed_bit(opcode), RegistryTarget::decode(opcode))
//             }
//             _ => panic!("Unknown prefix instruction code encountered: {:X}", opcode),
//         }
//     }
// }
//
// #[derive(Debug, Copy, Clone)]
// pub enum RegistryTarget {
//     B = 0x0,
//     C = 0x1,
//     D = 0x2,
//     E = 0x3,
//     H = 0x4,
//     L = 0x5,
//     HL = 0x6,
//     A = 0x7,
// }
//
// #[derive(Debug, Copy, Clone)]
// pub enum LoadByteSource {
//     A,
//     B,
//     C,
//     D,
//     E,
//     H,
//     L,
//     DirectU8,
//     HL,
// }
//
// #[derive(Debug, Copy, Clone)]
// pub enum LoadInfo {
//     Byte {
//         destination: RegistryTarget,
//         source: LoadByteSource,
//     },
// }
//
// #[derive(Debug, Copy, Clone)]
// pub enum JumpModifier {
//     NotZero,
//     Zero,
//     NotCarry,
//     Carry,
//     Always,
//     HL,
// }
//
// fn decode_prefixed_bit(opcode: u8) -> u8 {
//     let relevant_nibble = (opcode & 0xF0) % 0x4;
//     let lower_nibble = opcode & 0x0F;
//     match relevant_nibble {
//         0x0 if lower_nibble > 7 => 1,
//         0x0 => 0,
//         0x1 if lower_nibble > 7 => 3,
//         0x1 => 2,
//         0x2 if lower_nibble > 7 => 5,
//         0x2 => 4,
//         0x3 if lower_nibble > 7 => 7,
//         0x3 => 6,
//         _ => panic!(
//             "Encountered out of scope bit for relevant nib: {} and lower nib {}",
//             relevant_nibble, lower_nibble
//         ),
//     }
// }
//
// impl LoadInfo {
//     pub fn decode(opcode: u8) -> Self {
//         Self::Byte {
//             source: LoadByteSource::decode(opcode),
//             destination: RegistryTarget::decode_vertical(opcode),
//         }
//     }
// }
//
// impl LoadByteSource {
//     pub fn decode(opcode: u8) -> Self {
//         let relevant_nibble = (opcode & 0x0F) % 0x8;
//         match relevant_nibble {
//             0x0 => LoadByteSource::B,
//             0x1 => LoadByteSource::C,
//             0x2 => LoadByteSource::D,
//             0x3 => LoadByteSource::E,
//             0x4 => LoadByteSource::H,
//             0x5 => LoadByteSource::L,
//             0x6 => LoadByteSource::HL,
//             0x7 => LoadByteSource::A,
//             // This should never be called, unless maths has broken down.
//             _ => panic!("Invalid Nibble found: {:X}", relevant_nibble),
//         }
//     }
// }
//
// impl RegistryTarget {
//     pub fn decode(opcode: u8) -> Self {
//         let relevant_nibble = (opcode & 0x0F) % 0x8;
//         match relevant_nibble {
//             0x0 => RegistryTarget::B,
//             0x1 => RegistryTarget::C,
//             0x2 => RegistryTarget::D,
//             0x3 => RegistryTarget::E,
//             0x4 => RegistryTarget::H,
//             0x5 => RegistryTarget::L,
//             0x6 => RegistryTarget::HL,
//             0x7 => RegistryTarget::A,
//             // This should never be called, unless maths has broken down.
//             _ => panic!("Invalid Nibble found: {:X}", relevant_nibble),
//         }
//     }
//
//     pub fn decode_vertical(opcode: u8) -> Self {
//         let relevant_nibble = opcode & 0xF0;
//         let lower_nibble = opcode & 0x0F;
//         match relevant_nibble {
//             0x4 if lower_nibble < 0x8 => RegistryTarget::B,
//             0x4 if lower_nibble >= 0x8 => RegistryTarget::C,
//             0x5 if lower_nibble < 0x8 => RegistryTarget::D,
//             0x5 if lower_nibble >= 0x8 => RegistryTarget::E,
//             0x6 if lower_nibble < 0x8 => RegistryTarget::H,
//             0x6 if lower_nibble >= 0x8 => RegistryTarget::L,
//             0x7 if lower_nibble < 0x8 => RegistryTarget::HL,
//             0x7 if lower_nibble >= 0x8 => RegistryTarget::A,
//             _ => panic!("Invalid Nibble found: {:X}", relevant_nibble),
//         }
//     }
// }
// // Legacy get_next_instruction function
// ///// Fetches the next instruction.
// //     /// Modifies the `opcode` value, as well as advances the `PC` as necessary
// //     pub fn get_next_instruction(&mut self) -> Instruction {
// //         self.opcode = self.memory.read_byte(self.registers.pc);
// //         let instruction;
// //
// //         if self.opcode != 0xCB {
// //             instruction = Instruction::decode(self.opcode);
// //         } else {
// //             self.registers.pc.wrapping_add(1);
// //             instruction = Instruction::decode_prefix(self.memory.read_byte(self.registers.pc + 1));
// //         }
// //
// //         self.registers.pc.wrapping_add(1);
// //
// //         instruction
// //     }

// Execute the provided Instruction, note this does *not* automatically increment the `PC`
// unless done so by an instruction itself.
// pub fn execute(&mut self, instruction: Instruction) {
//     match instruction {
//         Instruction::NOP => return,
//         Instruction::HALT => self.halt(),
//         Instruction::ADD(target) => self.add(target),
//         Instruction::SUB(target) => self.sub(target),
//         Instruction::JP(condition) => self.jump(condition),
//         _ => debug!("Unimplemented instruction: {:?}", instruction),
//     }
// }

#[derive(Debug, Copy, Clone)]
pub enum RegistryTarget {
    B = 0x0,
    C = 0x1,
    D = 0x2,
    E = 0x3,
    H = 0x4,
    L = 0x5,
    HLI = 0x6,
    A = 0x7,
}

impl RegistryTarget {
    pub fn decode(opcode: u8) -> Self {
        let relevant_nibble = (opcode & 0x0F) % 0x8;
        match relevant_nibble {
            0x0 => RegistryTarget::B,
            0x1 => RegistryTarget::C,
            0x2 => RegistryTarget::D,
            0x3 => RegistryTarget::E,
            0x4 => RegistryTarget::H,
            0x5 => RegistryTarget::L,
            0x6 => RegistryTarget::HLI,
            0x7 => RegistryTarget::A,
            // This should never be called, unless maths has broken down.
            _ => panic!("Invalid Nibble found: {:X}", relevant_nibble),
        }
    }

    pub fn decode_vertical(opcode: u8) -> Self {
        let relevant_nibble = opcode & 0xF0;
        let lower_nibble = opcode & 0x0F;
        match relevant_nibble {
            0x4 if lower_nibble < 0x8 => RegistryTarget::B,
            0x4 if lower_nibble >= 0x8 => RegistryTarget::C,
            0x5 if lower_nibble < 0x8 => RegistryTarget::D,
            0x5 if lower_nibble >= 0x8 => RegistryTarget::E,
            0x6 if lower_nibble < 0x8 => RegistryTarget::H,
            0x6 if lower_nibble >= 0x8 => RegistryTarget::L,
            0x7 if lower_nibble < 0x8 => RegistryTarget::HLI,
            0x7 if lower_nibble >= 0x8 => RegistryTarget::A,
            _ => panic!("Invalid Nibble found: {:X}", relevant_nibble),
        }
    }
}
