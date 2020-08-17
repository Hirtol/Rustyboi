use crate::hardware::cpu::execute::{InstructionAddress, JumpModifier};
use crate::hardware::cpu::execute::InstructionAddress::HLI;
use crate::hardware::cpu::instructions::{Instruction, RegistryTarget};
use crate::hardware::cpu::CPU;
use crate::hardware::registers::{Flags, Reg16::*, Reg8::*};

#[test]
fn test_load_16bit() {
    use crate::hardware::registers::Reg16::*;
    use InstructionAddress::*;
    let mut cpu = initial_cpu();

    // Test register load

    cpu.registers.sp = 0x200;
    cpu.registers.set_hl(0x500);

    cpu.load_16bit(SP, HL);

    assert_eq!(cpu.registers.sp, 0x500);

    // Test mem -> registry load.

    cpu.registers.pc = 1;
    cpu.memory.set_short(1, 0x0105);

    cpu.load_16bit(BC, DIRECT);

    assert_eq!(cpu.registers.bc(), 0x0105);

    // Test cycle

    cpu.registers.pc = 0;
    cpu.memory.set_byte(0, 0x8);

    cpu.step_cycle();

    assert_eq!(cpu.memory.read_short(0x0105), 0x500);
    assert_eq!(cpu.registers.pc, 3);
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

#[test]
fn test_increment() {
    let mut cpu = initial_cpu();

    cpu.registers.c = 20;

    cpu.increment(C);

    assert_eq!(cpu.registers.c, 21);

    assert_eq!(cpu.memory.read_byte(0), 0);

    cpu.increment(HLI);

    assert_eq!(cpu.memory.read_byte(0), 1);
}

#[test]
fn test_increment_flags() {
    let mut cpu = initial_cpu();

    cpu.registers.a = 15;

    cpu.increment(A);

    assert_eq!(cpu.registers.a, 16);
    assert!(cpu.registers.f.contains(Flags::H));

    cpu.registers.b = 255;

    cpu.increment(B);

    assert_eq!(cpu.registers.b, 0);
    assert!(cpu.registers.f.contains(Flags::ZF));
}

#[test]
fn test_increment_16() {
    let mut cpu = initial_cpu();

    cpu.registers.set_bc(50);

    cpu.increment16(BC);

    assert_eq!(cpu.registers.bc(), 51);
}

#[test]
fn test_rlca() {
    let mut cpu = initial_cpu();

    cpu.registers.a = 0b0100_0101;

    cpu.rlca();

    assert_eq!(cpu.registers.a, 0b1000_1010);
    assert!(!cpu.registers.f.contains(Flags::CF));

    cpu.rlca();

    assert_eq!(cpu.registers.a, 0b0001_0101);
    assert!(cpu.registers.f.contains(Flags::CF));
}

#[test]
fn test_add_16bit() {
    let mut cpu = initial_cpu();

    cpu.registers.set_bc(0x0FFF);

    cpu.add_16bit(BC);

    assert_eq!(cpu.registers.hl(), 0x0FFF);
    assert!(!cpu.registers.f.contains(Flags::H));

    cpu.registers.set_de(0x001);

    cpu.add_16bit(DE);

    assert_eq!(cpu.registers.hl(), 0x1000);
    assert!(cpu.registers.f.contains(Flags::H));
}

#[test]
fn test_decrement() {
    let mut cpu = initial_cpu();
    cpu.registers.a = 5;

    cpu.decrement(A);

    assert_eq!(cpu.registers.a, 4);

    cpu.registers.set_hl(0x100);
    cpu.memory.set_byte(0x100, 1);
    cpu.decrement(HLI);

    assert_eq!(cpu.memory.read_byte(0x100), 0);
    assert!(cpu.registers.f.contains(Flags::ZF));
}

#[test]
fn test_decrement_16bit() {
    let mut cpu = initial_cpu();

    cpu.registers.set_bc(10);

    cpu.decrement16(BC);

    assert_eq!(cpu.registers.bc(), 9);

    cpu.registers.set_bc(0);
    cpu.decrement16(BC);

    assert_eq!(cpu.registers.bc(), u16::max_value());
}

#[test]
fn test_rrca() {
    let mut cpu = initial_cpu();

    cpu.registers.a = 0b0100_0001;
    cpu.rrca();

    assert_eq!(cpu.registers.a, 0b1010_0000);
    assert!(cpu.registers.f.contains(Flags::CF));

    cpu.rrca();

    assert_eq!(cpu.registers.a, 0b0101_0000);
    assert!(!cpu.registers.f.contains(Flags::CF));
}

#[test]
fn test_rla() {
    let mut cpu = initial_cpu();

    cpu.registers.a = 0b0100_0101;

    cpu.registers.set_cf(true);
    cpu.rla();

    assert_eq!(cpu.registers.a, 0b1000_1011);
    assert!(!cpu.registers.f.contains(Flags::CF));

    cpu.rla();

    assert_eq!(cpu.registers.a, 0b0001_0110);
    assert!(cpu.registers.f.contains(Flags::CF));
}

#[test]
fn test_relative_jump() {
    let mut cpu = initial_cpu();

    cpu.memory.set_byte(0, 0x18); // JR code
    cpu.memory.set_byte(1, 30);

    cpu.step_cycle();

    // TODO: Check if relative jump should also jump relative to it's own execution size (2 bytes)
    assert_eq!(cpu.registers.pc, 32);
    let test: i8 = -20;
    cpu.memory.set_byte(32, 0x18); // JR code
    cpu.memory.set_byte(33, test as u8);

    cpu.step_cycle();

    assert_eq!(cpu.registers.pc, 14);

    cpu.memory.set_byte(14, 0x28); // JR z-flag code
    cpu.step_cycle();

    assert_eq!(cpu.registers.pc, 16);
}

#[test]
fn test_rra(){
    let mut cpu = initial_cpu();

    cpu.registers.a = 0b0100_0101;

    cpu.rra();

    println!("{:08b}", cpu.registers.a);

    assert_eq!(cpu.registers.a, 0b0010_0010);
    assert!(cpu.registers.f.contains(Flags::CF));

    cpu.rra();

    assert_eq!(cpu.registers.a, 0b1001_0001);
    assert!(!cpu.registers.f.contains(Flags::CF));
}

#[test]
fn test_daa() {
    let mut cpu = initial_cpu();

    cpu.registers.b = 0x03;
    cpu.registers.a = 0x03;

    cpu.add(B);
    cpu.daa();

    assert_eq!(cpu.registers.a, 0x06);

    cpu.registers.c = 0x06;

    cpu.add(C);
    cpu.daa();

    assert_eq!(cpu.registers.a, 0x12);

    cpu.registers.d = 0x90;

    cpu.add(D);
    cpu.daa();

    assert_eq!(cpu.registers.a, 0x02);
    assert!(cpu.registers.cf());
}

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

fn initial_cpu() -> CPU {
    CPU::new()
}
