use crate::hardware::cpu::CPU;
use std::rc::Rc;
use std::cell::RefCell;
use crate::hardware::registers::Registers;
use crate::hardware::memory::MemoryMapper;

mod instruction_tests;
mod cycle_tests;

// Common functionality for the tests.

#[derive(Debug)]
struct TestMemory {
    mem: Vec<u8>,
}

impl MemoryMapper for TestMemory {
    fn read_byte(&self, address: u16) -> u8 {
        self.mem[address as usize]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.mem[address as usize] = value
    }

    fn boot_rom_finished(&self) -> bool {
        false
    }
}

impl<T: MemoryMapper> CPU<T> {
    fn set_instruction(&mut self, code: u8) {
        self.mmu.borrow_mut().write_byte(0, code);
    }
}

fn initial_cpu() -> CPU<TestMemory> {
    let mmu  = Rc::new(RefCell::new(TestMemory{mem: vec![0; 0x10000]}));
    let mut cpu = CPU::new(&mmu);
    cpu.registers = Registers::new();
    cpu
}

pub fn read_short<T: MemoryMapper>(cpu: &CPU<T>, address: u16) -> u16 {
    let least_s_byte = cpu.mmu.borrow().read_byte(address) as u16;
    let most_s_byte = cpu.mmu.borrow().read_byte(address.wrapping_add(1)) as u16;

    (most_s_byte << 8) | least_s_byte
}

pub fn set_short<T: MemoryMapper>(cpu: &mut CPU<T>, address: u16, value: u16) {
    cpu.mmu.borrow_mut().write_byte(address, (value & 0xFF) as u8); // Least significant byte first.
    cpu.mmu.borrow_mut().write_byte(address.wrapping_add(1), ((value & 0xFF00) >> 8) as u8);
}



