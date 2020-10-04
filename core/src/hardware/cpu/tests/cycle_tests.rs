use crate::hardware::cpu::tests::{initial_cpu, set_short};

use crate::io::interrupts::{InterruptFlags, Interrupts};

#[test]
fn basic_cycle_test() {
    let mut cpu = initial_cpu();

    // ADD A,B [4 cycles]
    set_short(&mut cpu, 0x0, 0x0080);
    cpu.step_cycle();
    assert_eq!(cpu.cycles_performed, 4);
    // LD (u16), SP [20 cycles]
    set_short(&mut cpu, 0x1, 0x0008);
    set_short(&mut cpu, 0x2, 0x5555);
    cpu.step_cycle();

    assert_eq!(cpu.cycles_performed, 24);
    // ADD HL,BC [8 cycles]
    set_short(&mut cpu, 0x4, 0x0009);
    cpu.step_cycle();

    assert_eq!(cpu.cycles_performed, 32);
}

#[test]
fn test_interrupt_cycles() {
    let mut cpu = initial_cpu();

    cpu.interrupts_routine(InterruptFlags::TIMER);
    // The true interrupt routine would be 20 cycles, but due to
    // the way we handle interrupts in the get_next_opcode() function this is still satisfied.
    assert_eq!(cpu.cycles_performed, 16);
    assert_eq!(cpu.registers.pc, 0x50);
}
