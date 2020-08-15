#[derive(Debug)]
pub enum Instruction {
    NOP,
    LD(LoadInfo),
    HALT,
    ADD(RegistryTarget),
    ADC(RegistryTarget),
    SUB(RegistryTarget),
    SBC(RegistryTarget),
    AND(RegistryTarget),
    XOR(RegistryTarget),
    OR(RegistryTarget),
    CP(RegistryTarget),
    JP(JumpModifier),

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

impl Instruction {
    pub fn decode(opcode: u8) -> Self {
        match opcode {
            0x00 => Instruction::NOP,
            0x40..=0x75 => Instruction::LD(LoadInfo::decode(opcode)),
            0x76 => Instruction::HALT,
            0x77..=0x7F => Instruction::LD(LoadInfo::decode(opcode)),
            0x80..=0x87 => Instruction::ADD(RegistryTarget::decode(opcode)),
            0x88..=0x8F => Instruction::ADC(RegistryTarget::decode(opcode)),
            0x90..=0x97 => Instruction::SUB(RegistryTarget::decode(opcode)),
            0x98..=0x9F => Instruction::SBC(RegistryTarget::decode(opcode)),
            0xA0..=0xA7 => Instruction::AND(RegistryTarget::decode(opcode)),
            0xA8..=0xAF => Instruction::XOR(RegistryTarget::decode(opcode)),
            0xB0..=0xB7 => Instruction::OR(RegistryTarget::decode(opcode)),
            0xB8..=0xBF => Instruction::CP(RegistryTarget::decode(opcode)),
            0xC2 => Instruction::JP(JumpModifier::NotZero),
            0xC3 => Instruction::JP(JumpModifier::Always),
            0xCA => Instruction::JP(JumpModifier::Zero),
            0xD2 => Instruction::JP(JumpModifier::NotCarry),
            0xDA => Instruction::JP(JumpModifier::Carry),
            0xE9 => Instruction::JP(JumpModifier::HL),
            _ => panic!("Unknown instruction code encountered: {:X}", opcode),
        }
    }

    pub fn decode_prefix(opcode: u8) -> Self {
        match opcode {
            0x00..=0x07 => Instruction::RLC(RegistryTarget::decode(opcode)),
            0x08..=0x0F => Instruction::RRC(RegistryTarget::decode(opcode)),
            0x10..=0x17 => Instruction::RL(RegistryTarget::decode(opcode)),
            0x18..=0x1F => Instruction::RR(RegistryTarget::decode(opcode)),
            0x20..=0x27 => Instruction::SLA(RegistryTarget::decode(opcode)),
            0x28..=0x2F => Instruction::SRA(RegistryTarget::decode(opcode)),
            0x30..=0x37 => Instruction::SWAP(RegistryTarget::decode(opcode)),
            0x38..=0x3F => Instruction::SRL(RegistryTarget::decode(opcode)),
            0x40..=0x7F => {
                Instruction::BIT(decode_prefixed_bit(opcode), RegistryTarget::decode(opcode))
            }
            0x80..=0xBF => {
                Instruction::RES(decode_prefixed_bit(opcode), RegistryTarget::decode(opcode))
            }
            0xC0..=0xFF => {
                Instruction::SET(decode_prefixed_bit(opcode), RegistryTarget::decode(opcode))
            }
            _ => panic!("Unknown prefix instruction code encountered: {:X}", opcode),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum RegistryTarget {
    B = 0x0,
    C = 0x1,
    D = 0x2,
    E = 0x3,
    H = 0x4,
    L = 0x5,
    HL = 0x6,
    A = 0x7,
}

#[derive(Debug, Copy, Clone)]
pub enum LoadByteSource {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    DirectU8,
    HL,
}

#[derive(Debug, Copy, Clone)]
pub enum LoadInfo {
    Byte {
        destination: RegistryTarget,
        source: LoadByteSource,
    },
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

fn decode_prefixed_bit(opcode: u8) -> u8 {
    let relevant_nibble = (opcode & 0xF0) % 0x4;
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

impl LoadInfo {
    pub fn decode(opcode: u8) -> Self {
        Self::Byte {
            source: LoadByteSource::decode(opcode),
            destination: RegistryTarget::decode_vertical(opcode),
        }
    }
}

impl LoadByteSource {
    pub fn decode(opcode: u8) -> Self {
        let relevant_nibble = (opcode & 0x0F) % 0x8;
        match relevant_nibble {
            0x0 => LoadByteSource::B,
            0x1 => LoadByteSource::C,
            0x2 => LoadByteSource::D,
            0x3 => LoadByteSource::E,
            0x4 => LoadByteSource::H,
            0x5 => LoadByteSource::L,
            0x6 => LoadByteSource::HL,
            0x7 => LoadByteSource::A,
            // This should never be called, unless maths has broken down.
            _ => panic!("Invalid Nibble found: {:X}", relevant_nibble),
        }
    }
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
            0x6 => RegistryTarget::HL,
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
            0x7 if lower_nibble < 0x8 => RegistryTarget::HL,
            0x7 if lower_nibble >= 0x8 => RegistryTarget::A,
            _ => panic!("Invalid Nibble found: {:X}", relevant_nibble),
        }
    }
}
