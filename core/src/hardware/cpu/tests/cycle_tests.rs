use crate::hardware::cpu::tests::{initial_cpu, set_short, read_short};
use crate::hardware::memory::MemoryMapper;

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

    set_short(&mut cpu, 0x4,0x0009);
    cpu.step_cycle();

    assert_eq!(cpu.cycles_performed, 32);
}