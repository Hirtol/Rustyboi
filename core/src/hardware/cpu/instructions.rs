#[derive(Debug, PartialOrd, PartialEq)]
pub enum Instruction{
    ADD(RegistryTarget),
    ADC(RegistryTarget),
    SUB(RegistryTarget),
}

#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
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

impl Instruction {
    pub fn decode(opcode: u8) -> Self {
        match opcode {
            0x80..=0x87 => Instruction::ADD(RegistryTarget::decode(opcode)),
            0x88..=0x8F => Instruction::ADC(RegistryTarget::decode(opcode)),
            0x90..=0x97 => Instruction::SUB(RegistryTarget::decode(opcode)),
            _ => panic!("Unknown instruction code encountered: {:X}", opcode),
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
            _ => panic!("Invalid Nibble found: {:X}", relevant_nibble)
        }
    }
}