use crate::hardware::cartridge::Cartridge;
use crate::hardware::cpu::CPU;
use crate::hardware::memory::MemoryMapper;
use crate::hardware::registers::Registers;
use std::cell::RefCell;
use std::rc::Rc;
use crate::io::interrupts::InterruptModule;
use crate::hardware::ppu::PPU;
use crate::hardware::apu::APU;
use crate::io::timer::TimerRegisters;
use bitflags::_core::fmt::{Debug, Formatter};
use std::fmt;

mod cycle_tests;
mod instruction_tests;

// Common functionality for the tests.

struct TestMemory {
    mem: Vec<u8>,
    pub ppu: PPU,
    pub apu: APU,
    pub timers: TimerRegisters,
    pub interrupts: InterruptModule,
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

    fn cartridge(&self) -> Option<&Cartridge> {
        None
    }

    fn interrupts(&self) -> &InterruptModule {
        &self.interrupts
    }

    fn interrupts_mut(&mut self) -> &mut InterruptModule {
        &mut self.interrupts
    }

    fn ppu_mut(&mut self) -> &mut PPU {
        &mut self.ppu
    }

    fn apu_mut(&mut self) -> &mut APU {
        &mut self.apu
    }

    fn timers_mut(&mut self) -> &mut TimerRegisters {
        &mut self.timers
    }
}

impl Debug for TestMemory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unimplemented!()
    }
}

impl<T: MemoryMapper> CPU<T> {
    fn set_instruction(&mut self, code: u8) {
        self.mmu.write_byte(0, code);
    }
}

fn initial_cpu() -> CPU<TestMemory> {
    let mut cpu = CPU::new(TestMemory { mem: vec![0; 0x10000], ppu: PPU::new(), apu: APU::new(), timers: Default::default(), interrupts: Default::default() });
    cpu.registers = Registers::new();
    cpu
}

pub fn read_short<T: MemoryMapper>(cpu: &CPU<T>, address: u16) -> u16 {
    let least_s_byte = cpu.mmu.read_byte(address) as u16;
    let most_s_byte = cpu.mmu.read_byte(address.wrapping_add(1)) as u16;

    (most_s_byte << 8) | least_s_byte
}

pub fn set_short<T: MemoryMapper>(cpu: &mut CPU<T>, address: u16, value: u16) {
    cpu.mmu.write_byte(address, (value & 0xFF) as u8); // Least significant byte first.
    cpu.mmu
        .write_byte(address.wrapping_add(1), ((value & 0xFF00) >> 8) as u8);
}
