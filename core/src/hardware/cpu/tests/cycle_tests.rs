use crate::hardware::cpu::tests::{initial_cpu, read_short, set_short};
use crate::hardware::memory::MemoryMapper;
use crate::io::interrupts::Interrupts;

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

    set_short(&mut cpu, 0x4, 0x0009);
    cpu.step_cycle();

    assert_eq!(cpu.cycles_performed, 32);
}

#[test]
fn test_interrupt_cycles() {
    let mut cpu = initial_cpu();

    cpu.interrupts_routine(Interrupts::TIMER);

    assert_eq!(cpu.cycles_performed, 20);
    assert_eq!(cpu.registers.pc, 0x50);
}
