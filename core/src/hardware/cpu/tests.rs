use crate::hardware::cpu::instructions::{Instruction, RegistryTarget};
use crate::hardware::cpu::CPU;
use crate::hardware::registers::Flags;

#[test]
fn test_add() {
    let mut cpu = initial_cpu();
    // Test normal add
    cpu.registers.a = 10;
    cpu.registers.c = 20;

    cpu.execute(Instruction::ADD(RegistryTarget::C));

    assert_eq!(cpu.registers.a, 30);

    // Test overflow
    cpu.registers.c = 230;

    cpu.execute(Instruction::ADD(RegistryTarget::C));

    assert_eq!(cpu.registers.a, 4);
    assert!(cpu.registers.f.contains(Flags::CF));
}

fn initial_cpu() -> CPU {
    CPU::new()
}
