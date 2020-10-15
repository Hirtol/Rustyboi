use crate::hardware::apu::APU;
use crate::hardware::cartridge::Cartridge;
use crate::hardware::cpu::CPU;
use crate::hardware::mmu::MemoryMapper;
use crate::hardware::ppu::PPU;
use crate::hardware::cpu::registers::Registers;
use crate::io::interrupts::Interrupts;
use crate::io::timer::TimerRegisters;
use crate::scheduler::Scheduler;
use bitflags::_core::fmt::{Debug, Formatter};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use crate::emulator::EmulatorMode;
use crate::emulator::EmulatorMode::DMG;
use crate::hardware::mmu::cgb_mem::CgbSpeedData;
use crate::hardware::ppu::palette::DisplayColour;

mod cycle_tests;
mod instruction_tests;

// Common functionality for the tests.

struct TestMemory {
    mem: Vec<u8>,
    pub ppu: PPU,
    pub apu: APU,
    pub timers: TimerRegisters,
    pub interrupts: Interrupts,
    pub cgb_data: CgbSpeedData
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

    fn get_mode(&self) -> EmulatorMode {
        DMG
    }

    fn cartridge(&self) -> Option<&Cartridge> {
        None
    }

    fn interrupts(&self) -> &Interrupts {
        &self.interrupts
    }

    fn interrupts_mut(&mut self) -> &mut Interrupts {
        &mut self.interrupts
    }

    fn turn_on_lcd(&mut self) {
        unimplemented!()
    }

    fn turn_off_lcd(&mut self) {
        unimplemented!()
    }

    fn cgb_data(&mut self) -> &mut CgbSpeedData {
        &mut self.cgb_data
    }

    fn do_m_cycle(&mut self) -> bool {
        false
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
    let mut cpu = CPU::new(TestMemory {
        mem: vec![0; 0x10000],
        ppu: PPU::new(DisplayColour::default()),
        apu: APU::new(),
        timers: Default::default(),
        interrupts: Default::default(),
        cgb_data: Default::default()
    });
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
