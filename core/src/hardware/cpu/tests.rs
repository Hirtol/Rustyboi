use crate::hardware::cpu::execute::InstructionAddress;
use crate::hardware::cpu::instructions::{Instruction, RegistryTarget};
use crate::hardware::cpu::CPU;
use crate::hardware::registers::{Flags, Reg8::*};

#[test]
fn test_add() {
    let mut cpu = initial_cpu();
    // Test normal add
    cpu.registers.a = 10;
    cpu.registers.c = 20;

    cpu.add(C);

    assert_eq!(cpu.registers.a, 30);

    // Test overflow
    cpu.registers.c = 230;

    cpu.add(C);

    assert_eq!(cpu.registers.a, 4);
    assert!(cpu.registers.f.contains(Flags::CF));
}

#[test]
fn test_load_8bit() {
    let mut cpu = initial_cpu();

    cpu.registers.c = 40;
    cpu.registers.set_hl(0x4000);
    cpu.memory.set_byte(0x4000, 30);

    // Basic test
    cpu.load_8bit(B, C);

    assert_eq!(cpu.registers.b, 40);

    // Test from memory
    cpu.load_8bit(D, InstructionAddress::HLI);

    assert_eq!(cpu.registers.d, 30);

    // Test to memory
    cpu.load_8bit(InstructionAddress::HLI, C);

    assert_eq!(cpu.memory.read_byte(cpu.registers.hl()), 40);

    // Test if execute can handle some instructions.
    cpu.execute(0x7A);

    assert_eq!(cpu.registers.a, 30);
}

fn initial_cpu() -> CPU {
    CPU::new()
}
