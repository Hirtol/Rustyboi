#[derive(Debug)]
pub enum Instruction {
    ADD(RegistryTarget),
    ADC(RegistryTarget),
    SUB(RegistryTarget),
    SBC(RegistryTarget),
    AND(RegistryTarget),
    XOR(RegistryTarget),
    OR(RegistryTarget),
    CP(RegistryTarget),

}

impl Instruction {
    pub fn decode(opcode: u8) -> Self {
        match opcode {
            0x80..=0x87 => Instruction::ADD(RegistryTarget::decode(opcode)),
            0x88..=0x8F => Instruction::ADC(RegistryTarget::decode(opcode)),
            0x90..=0x97 => Instruction::SUB(RegistryTarget::decode(opcode)),
            0x98..=0x9F => Instruction::SBC(RegistryTarget::decode(opcode)),
            0xA0..=0xA7 => Instruction::AND(RegistryTarget::decode(opcode)),
            0xA8..=0xAF => Instruction::XOR(RegistryTarget::decode(opcode)),
            0xB0..=0xB7 => Instruction::OR(RegistryTarget::decode(opcode)),
            0xB8..=0xBF => Instruction::CP(RegistryTarget::decode(opcode)),
            _ => panic!("Unknown instruction code encountered: {:X}", opcode),
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
            _ => panic!("Invalid Nibble found: {:X}", relevant_nibble)
        }
    }
}