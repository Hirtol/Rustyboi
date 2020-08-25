use crate::emulator::MMU;
use crate::hardware::cpu::execute::InstructionAddress::HLI;
use crate::hardware::cpu::execute::{InstructionAddress, JumpModifier};
use crate::hardware::cpu::instructions::{Instruction, RegistryTarget};
use crate::hardware::cpu::tests::{initial_cpu, read_short, set_short};
use crate::hardware::cpu::CPU;
use crate::hardware::memory::{Memory, MemoryMapper};
use crate::hardware::registers::{Flags, Reg16::*, Reg8::*, Registers};
use crate::io::bootrom::BootRom;
use bitflags::_core::cell::RefCell;
use std::rc::Rc;

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
    set_short(&mut cpu, 1, 0x0105);

    cpu.load_16bit(BC, DIRECT);

    assert_eq!(cpu.registers.bc(), 0x0105);

    // Test cycle

    cpu.registers.pc = 0;
    cpu.mmu.borrow_mut().write_byte(0, 0x8);
    cpu.cycles_performed = 0;
    cpu.step_cycle();

    assert_eq!(read_short(&cpu, 0x0105), 0x500);
    assert_eq!(cpu.cycles_performed, 20);
    assert_eq!(cpu.registers.pc, 3);
}

#[test]
fn test_load_8bit() {
    let mut cpu = initial_cpu();

    cpu.registers.c = 40;
    cpu.registers.set_hl(0x4000);
    cpu.mmu.borrow_mut().write_byte(0x4000, 30);

    // Basic test
    cpu.load_8bit(B, C);

    assert_eq!(cpu.registers.b, 40);

    // Test from memory
    cpu.load_8bit(D, InstructionAddress::HLI);

    assert_eq!(cpu.registers.d, 30);

    // Test to memory
    cpu.load_8bit(InstructionAddress::HLI, C);

    assert_eq!(cpu.mmu.borrow().read_byte(cpu.registers.hl()), 40);

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

    assert_eq!(cpu.mmu.borrow().read_byte(0), 0);

    cpu.increment(HLI);

    assert_eq!(cpu.mmu.borrow().read_byte(0), 1);
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

    cpu.add16(BC);

    assert_eq!(cpu.registers.hl(), 0x0FFF);
    assert!(!cpu.registers.f.contains(Flags::H));

    cpu.registers.set_de(0x001);

    cpu.add16(DE);

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
    cpu.mmu.borrow_mut().write_byte(0x100, 1);
    cpu.decrement(HLI);

    assert_eq!(cpu.mmu.borrow().read_byte(0x100), 0);
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

    cpu.mmu.borrow_mut().write_byte(0, 0x18); // JR code
    cpu.mmu.borrow_mut().write_byte(1, 30);

    cpu.step_cycle();

    // TODO: Check if relative jump should also jump relative to it's own execution size (2 bytes)
    assert_eq!(cpu.registers.pc, 32);
    let test: i8 = -20;
    cpu.mmu.borrow_mut().write_byte(32, 0x18); // JR code
    cpu.mmu.borrow_mut().write_byte(33, test as u8);

    cpu.step_cycle();

    assert_eq!(cpu.registers.pc, 14);

    cpu.mmu.borrow_mut().write_byte(14, 0x28); // JR z-flag code
    cpu.step_cycle();

    assert_eq!(cpu.registers.pc, 16);
}

#[test]
fn test_rra() {
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
fn test_cpl() {
    let mut cpu = initial_cpu();

    cpu.registers.a = 0b1010_0101;

    cpu.cpl();

    assert_eq!(cpu.registers.a, 0b0101_1010);
    assert!(cpu.registers.hf());
    assert!(cpu.registers.n());
}

#[test]
fn test_scf() {
    let mut cpu = initial_cpu();

    cpu.scf();

    assert!(cpu.registers.cf());
    assert!(!cpu.registers.hf());
    assert!(!cpu.registers.n());
}

#[test]
fn test_ccf() {
    let mut cpu = initial_cpu();

    cpu.ccf();

    assert!(cpu.registers.cf());
    assert!(!cpu.registers.hf());
    assert!(!cpu.registers.n());

    cpu.ccf();

    assert!(!cpu.registers.cf());
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

#[test]
fn test_adc() {
    let mut cpu = initial_cpu();
    // Test carry add
    cpu.registers.a = 10;
    cpu.registers.c = 20;
    cpu.registers.set_cf(true);

    cpu.adc(C);

    assert_eq!(cpu.registers.a, 31);
}

#[test]
fn test_sub() {
    let mut cpu = initial_cpu();
    // Test normal add
    cpu.registers.a = 20;
    cpu.registers.c = 10;

    cpu.sub(C);

    assert_eq!(cpu.registers.a, 10);

    // Test overflow
    cpu.registers.c = 16;

    cpu.sub(C);

    assert_eq!(cpu.registers.a, 250);
    assert!(cpu.registers.cf());
    assert!(!cpu.registers.hf());

    cpu.registers.a = 0b0001_0101;
    cpu.registers.b = 0b0000_1000;

    cpu.sub(B);

    assert!(cpu.registers.hf())
}

#[test]
fn test_sbc() {
    let mut cpu = initial_cpu();
    // Test carry add
    cpu.registers.a = 20;
    cpu.registers.c = 10;
    cpu.registers.set_cf(true);

    cpu.sbc(C);

    assert_eq!(cpu.registers.a, 9);
}

#[test]
fn test_and() {
    let mut cpu = initial_cpu();
    // Test carry add
    cpu.registers.a = 0b1000_1010;
    cpu.registers.c = 0b1111_0011;

    cpu.and(C);

    assert_eq!(cpu.registers.a, 0b1000_0010);
}

#[test]
fn test_xor() {
    let mut cpu = initial_cpu();
    // Test carry add
    cpu.registers.a = 0b0000_1010;
    cpu.registers.c = 0b1111_0011;

    cpu.xor(C);

    assert_eq!(cpu.registers.a, 0b1111_1001);
}

#[test]
fn test_or() {
    let mut cpu = initial_cpu();
    // Test carry add
    cpu.registers.a = 0b0000_1010;
    cpu.registers.c = 0b1111_0011;

    cpu.or(C);

    assert_eq!(cpu.registers.a, 0b1111_1011);
}

#[test]
fn test_compare() {
    let mut cpu = initial_cpu();
    // Test overflow
    cpu.registers.c = 16;
    cpu.registers.a = 10;

    cpu.compare(C);

    assert_eq!(cpu.registers.a, 10);
    assert!(cpu.registers.cf());
    assert!(!cpu.registers.hf());

    cpu.registers.a = 0b0001_0101;
    cpu.registers.b = 0b0000_1000;

    cpu.compare(B);

    assert!(cpu.registers.hf())
}

#[test]
fn test_call_and_ret() {
    let mut cpu = initial_cpu();
    cpu.registers.sp = 0xFFFF;
    cpu.mmu.borrow_mut().write_byte(0, 0xCD);
    set_short(&mut cpu, 1, 0x1445);

    cpu.step_cycle();

    assert_eq!(cpu.registers.pc, 0x1445);
    assert_eq!(cpu.registers.sp, 0xFFFD);
    // Previous PC
    assert_eq!(read_short(&cpu, 0xFFFD), 3);

    cpu.ret(JumpModifier::Always);

    assert_eq!(cpu.registers.pc, 3);
}

#[test]
fn test_push_and_pop() {
    let mut cpu = initial_cpu();
    cpu.registers.set_bc(0x500);
    cpu.registers.sp = 0xFFFF;

    cpu.push(BC);

    assert_eq!(cpu.registers.sp, 0xFFFD);
    assert_eq!(read_short(&cpu, cpu.registers.sp), 0x500);

    cpu.pop(DE);

    assert_eq!(cpu.registers.de(), 0x500);
}

#[test]
fn test_rst() {
    let mut cpu = initial_cpu();
    cpu.registers.sp = 0xFFFF;

    cpu.set_instruction(0xFF);

    cpu.step_cycle();

    assert_eq!(cpu.registers.pc, 0x38);
}

#[test]
fn test_add_sp() {
    let mut cpu = initial_cpu();
    cpu.registers.sp = 50;

    cpu.set_instruction(0xE8);
    cpu.mmu.borrow_mut().write_byte(1, (-20 as i8) as u8);

    cpu.step_cycle();

    assert_eq!(cpu.registers.sp, 30);
    assert_eq!(cpu.registers.pc, 2);
    //TODO: Add flag tests once we figure out what they actually are supposed to be ._.
}

#[test]
fn test_load_sp() {
    let mut cpu = initial_cpu();
    cpu.registers.sp = 50;
    cpu.set_instruction((-30 as i8) as u8);

    cpu.load_sp_i();

    assert_eq!(cpu.registers.hl(), 20);
    assert_eq!(cpu.registers.sp, 50);
    //TODO: Add flag tests once we figure out what they actually are supposed to be ._.
}

#[test]
fn test_rlc() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.rlc(B);

    assert_eq!(cpu.registers.b, 0b0101_0011);
    assert!(cpu.registers.cf());

    cpu.rlc(B);

    assert_eq!(cpu.registers.b, 0b1010_0110);
    assert!(!cpu.registers.cf())
}

#[test]
fn test_rrc() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.rrc(B);

    assert_eq!(cpu.registers.b, 0b1101_0100);
    assert!(cpu.registers.cf());

    cpu.rrc(B);

    assert_eq!(cpu.registers.b, 0b0110_1010);
    assert!(!cpu.registers.cf());
}

#[test]
fn test_rl() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.rl(B);

    assert_eq!(cpu.registers.b, 0b0101_0010);
    assert!(cpu.registers.cf());

    cpu.rl(B);

    assert_eq!(cpu.registers.b, 0b1010_0101);
    assert!(!cpu.registers.cf())
}

#[test]
fn test_rr() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.rr(B);

    assert_eq!(cpu.registers.b, 0b0101_0100);
    assert!(cpu.registers.cf());

    cpu.rr(B);

    assert_eq!(cpu.registers.b, 0b1010_1010);
    assert!(!cpu.registers.cf());
}

#[test]
fn test_sla() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.sla(B);

    assert_eq!(cpu.registers.b, 0b0101_0010);
    assert!(cpu.registers.cf());

    cpu.sla(B);

    assert_eq!(cpu.registers.b, 0b1010_0100);
    assert!(!cpu.registers.cf())
}

#[test]
fn test_sra() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.sra(B);

    assert_eq!(cpu.registers.b, 0b1101_0100);
    assert!(cpu.registers.cf());

    cpu.sra(B);

    assert_eq!(cpu.registers.b, 0b1110_1010);
    assert!(!cpu.registers.cf());

    cpu.registers.c = 0b0110_0110;

    cpu.sra(C);

    assert_eq!(cpu.registers.c, 0b0011_0011);
}

#[test]
fn test_swap() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.swap(B);

    assert_eq!(cpu.registers.b, 0b1001_1010);

    cpu.swap(B);

    assert_eq!(cpu.registers.b, 0b1010_1001);
}

#[test]
fn test_srl() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.srl(B);

    assert_eq!(cpu.registers.b, 0b0101_0100);
    assert!(cpu.registers.cf());

    cpu.srl(B);

    assert_eq!(cpu.registers.b, 0b0010_1010);
    assert!(!cpu.registers.cf());
}

#[test]
fn test_bit() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.bit(7, B);

    assert!(!cpu.registers.zf());

    cpu.bit(6, B);

    assert!(cpu.registers.zf());
}

#[test]
fn test_set() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.set(6, B);

    assert_eq!(cpu.registers.b, 0b1110_1001);
}

#[test]
fn test_res() {
    let mut cpu = initial_cpu();
    cpu.registers.b = 0b1010_1001;

    cpu.res(7, B);

    assert_eq!(cpu.registers.b, 0b0010_1001);
}
