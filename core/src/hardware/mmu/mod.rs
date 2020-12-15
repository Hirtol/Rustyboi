use std::fmt;

use bitflags::_core::fmt::{Debug, Formatter};
use bitflags::_core::ops::{Add, Mul, Sub};
use itertools::Itertools;
use log::*;

use hram::Hram;

use crate::gb_emu::GameBoyModel;
use crate::gb_emu::GameBoyModel::DMG;
use crate::hardware::apu::{
    APU, APU_MEM_END, APU_MEM_START, FRAME_SEQUENCE_CYCLES, SAMPLE_CYCLES, WAVE_SAMPLE_END, WAVE_SAMPLE_START,
};
use crate::hardware::cartridge::Cartridge;
use crate::hardware::mmu::cgb_mem::HdmaMode::HDMA;
use crate::hardware::mmu::cgb_mem::{CgbSpeedData, HdmaRegister};
use crate::hardware::mmu::wram::Wram;
use crate::hardware::ppu::tiledata::*;
use crate::hardware::ppu::{PPU, Mode};
use crate::io::bootrom::BootRom;
use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::scheduler::EventType::{DMARequested, DMATransferComplete};
use crate::scheduler::{Event, EventType, Scheduler};
use crate::EmulatorOptions;
use crate::hardware::ppu::timing::{OAM_SEARCH_DURATION, SCANLINE_DURATION};
use crate::hardware::ppu::memory_binds::DMA_TRANSFER;
use crate::io::joypad::JoyPad;
use crate::io::timer::{TimerRegisters, TIMER_COUNTER, TIMER_CONTROL, TIMER_MODULO};
use crate::io::io_registers::IORegisters;

pub mod cgb_mem;
mod dma;
mod hram;
mod wram;

/// 16 KB ROM bank, usually 00. From Cartridge, read-only
pub const ROM_BANK_00_START: u16 = 0x0000;
pub const ROM_BANK_00_END: u16 = 0x03FFF;
/// 16 KB Rom Bank 01~NN. From cartridge, switchable bank via Memory Bank. Read-only.
pub const ROM_BANK_NN_START: u16 = 0x4000;
pub const ROM_BANK_NN_END: u16 = 0x7FFF;
/// 8 KB of VRAM, only bank 0 in Non-CGB mode. Switchable bank 0/1 in CGB mode.
pub const VRAM_START: u16 = 0x8000;
pub const VRAM_END: u16 = 0x9FFF;
/// 8 KB of External Ram, In cartridge, switchable bank if any(?). Could hold save data.
pub const EXTERNAL_RAM_START: u16 = 0xA000;
pub const EXTERNAL_RAM_END: u16 = 0xBFFF;
/// 4 KB Work RAM bank 0
pub const WRAM_BANK_00_START: u16 = 0xC000;
pub const WRAM_BANK_00_END: u16 = 0xCFFF;
/// 4 KB Work RAM bank 1~N. Only bank 1 in Non-CGB mode Switchable bank 1~7 in CGB mode.
pub const WRAM_BANK_NN_START: u16 = 0xD000;
pub const WRAM_BANK_NN_END: u16 = 0xDFFF;
/// Mirror of C000~DDFF (ECHO RAM). Typically not used
pub const ECHO_RAM_START: u16 = 0xE000;
pub const ECHO_RAM_END: u16 = 0xFDFF;
/// Sprite attribute table (OAM)
pub const OAM_ATTRIBUTE_START: u16 = 0xFE00;
pub const OAM_ATTRIBUTE_END: u16 = 0xFE9F;

pub const NOT_USABLE_START: u16 = 0xFEA0;
pub const NOT_USABLE_END: u16 = 0xFEFF;
/// I/O Registers
pub const IO_START: u16 = 0xFF00;
pub const IO_END: u16 = 0xFF7F;

pub const JOYPAD_REGISTER: u16 = 0xFF00;
pub const SIO_DATA: u16 = 0xFF01;
/// FF02 -- SIOCONT [RW] Serial I/O Control       | when set to 1 | when set to 0
/// Bit7  Transfer start flag                     | START         | NO TRANSFER
/// Bit0  Serial I/O clock select                 | INTERNAL      | EXTERNAL
pub const SIO_CONT: u16 = 0xFF02;
/// This register is incremented at rate of 16384Hz (~16779Hz on SGB).
/// Writing any value to this register resets it to 00h.
///
/// Note: The divider is affected by CGB double speed mode, and will increment at 32768Hz in double speed.
pub const DIVIDER_REGISTER: u16 = 0xFF04;

pub const PPU_IO_START: u16 = 0xF40;
pub const PPU_IO_END: u16 = 0xFF4F;
pub const PPU_CGB_IO_START: u16 = 0xFF68;
pub const PPU_CGB_IO_END: u16 = 0xFF6C;
// TODO: Implement
/// Not documented anywhere I could find, but if one writes 0x04 to this register it'll manually
/// put the CGB into DMG mode (e.g, sprite priority changes)
pub const CGB_SWITCH_MODE: u16 = 0xFF4C;
pub const CGB_PREPARE_SWITCH: u16 = 0xFF4D;
/// Specifies the higher byte of the source address. Always returns FFh when read.
pub const CGB_HDMA_1: u16 = 0xFF51;
/// Specifies the lower byte of the source address. Lower 4 bits are ignored, addresses are always
/// aligned to 10h (16 bytes). Always returns FFh when read.
pub const CGB_HDMA_2: u16 = 0xFF52;
/// Specifies the higher byte of the destination address. Destination is always in VRAM (8000h –
/// 9FFFh), the 3 upper bits are ignored. Always returns FFh when read.
pub const CGB_HDMA_3: u16 = 0xFF53;
/// Specifies the lower byte of the destination address. Lower 4 bits are ignored, addresses are always
/// aligned to 10h (16 bytes). Always returns FFh when read.
pub const CGB_HDMA_4: u16 = 0xFF54;
/// This register specifies the length and mode of the transfer. It starts the copy when it is written.
/// Returns FFh in DMG and GBC in DMG mode.
/// Bit 7 – Transfer mode (0=GDMA, 1=HDMA)
/// Bits 6-0 – Blocks (Size = (Blocks+1)×16 bytes)
pub const CGB_HDMA_5: u16 = 0xFF55;
/// Infrared Communications Port //TODO: Implement?
pub const CGB_RP: u16 = 0xFF56;
/// Work ram bank switching.
pub const CGB_WRAM_BANK: u16 = 0xFF70;

/// The flag used to signal that an interrupt is pending.
pub const INTERRUPTS_FLAG: u16 = 0xFF0F;
/// High Ram (HRAM)
pub const HRAM_START: u16 = 0xFF80;
pub const HRAM_END: u16 = 0xFFFE;
/// Interrupts Enable Register (IE)
pub const INTERRUPTS_ENABLE: u16 = 0xFFFF;
/// The value to return for an invalid read
pub const INVALID_READ: u8 = 0xFF;

/// Simple memory interface for reading and writing bytes, as well as determining the
/// state of the BootRom.
pub trait MemoryMapper: Debug {
    fn read_byte(&mut self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);
    fn boot_rom_finished(&self) -> bool;
    fn get_mode(&self) -> GameBoyModel;
    /// Returns, if the current ROM has a battery, the contents of the External Ram.
    ///
    /// Should be used for saving functionality.
    fn cartridge(&self) -> Option<&Cartridge>;
    fn interrupts(&self) -> &Interrupts;
    fn interrupts_mut(&mut self) -> &mut Interrupts;
    fn turn_on_lcd(&mut self);
    fn turn_off_lcd(&mut self);
    fn cgb_data(&mut self) -> &mut CgbSpeedData;
    /// Perform one M-cycle (4 cycles) on all components of the system.
    /// Returns `true` if V-blank occurred
    fn do_m_cycle(&mut self) -> bool;
    /// Skip ahead to the next event, whenever that may be.
    /// Useful for halt skipping.
    fn execute_next_event(&mut self) -> bool;
}

pub struct Memory {
    boot_rom: BootRom,
    cartridge: Cartridge,
    pub scheduler: Scheduler,
    pub emulated_model: GameBoyModel,
    pub cgb_data: CgbSpeedData,
    pub hdma: HdmaRegister,

    pub ppu: PPU,
    pub apu: APU,
    pub hram: Hram,
    pub wram: Wram,

    pub joypad_register: JoyPad,
    pub timers: TimerRegisters,
    pub interrupts: Interrupts,
    pub io_registers: IORegisters,
}

impl Memory {
    pub fn new(rom_data: &[u8], emu_opts: EmulatorOptions) -> Self {
        let cartridge = Cartridge::new(rom_data, emu_opts.saved_ram);
        Memory {
            boot_rom: BootRom::new(emu_opts.boot_rom),
            ppu: PPU::new(
                emu_opts.bg_display_colour,
                emu_opts.sp0_display_colour,
                emu_opts.sp1_display_colour,
                cartridge.cartridge_header().cgb_flag && emu_opts.emulator_mode.is_cgb(),
                emu_opts.emulator_mode,
            ),
            cartridge,
            scheduler: Scheduler::new(),
            emulated_model: emu_opts.emulator_mode,
            cgb_data: CgbSpeedData::new(),
            hdma: HdmaRegister::new(),
            apu: APU::new(),
            hram: Hram::new(),
            wram: Wram::new(),
            joypad_register: JoyPad::new(),
            timers: Default::default(),
            interrupts: Default::default(),
            io_registers: IORegisters::new(),
        }
    }

    pub fn read_byte(&mut self, address: u16) -> u8 {
        match address {
            0x0000..=0x00FF if !self.boot_rom.is_finished => self.boot_rom.read_byte(address),
            0x0200..=0x08FF if !self.boot_rom.is_finished && self.emulated_model.is_cgb() => {
                self.boot_rom.read_byte(address)
            }
            ROM_BANK_00_START..=ROM_BANK_00_END => self.cartridge.read_0000_3fff(address),
            ROM_BANK_NN_START..=ROM_BANK_NN_END => self.cartridge.read_4000_7fff(address),
            VRAM_START..=VRAM_END => self.ppu.read_vram(address),
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => self.cartridge.read_external_ram(address),
            WRAM_BANK_00_START..=WRAM_BANK_00_END => self.wram.read_bank_0(address),
            WRAM_BANK_NN_START..=WRAM_BANK_NN_END => self.wram.read_bank_n(address),
            ECHO_RAM_START..=ECHO_RAM_END => self.wram.read_echo_ram(address),
            OAM_ATTRIBUTE_START..=OAM_ATTRIBUTE_END => self.ppu.read_vram(address),
            NOT_USABLE_START..=NOT_USABLE_END => self.non_usable_call(address),
            IO_START..=IO_END => self.read_io_byte(address),
            HRAM_START..=HRAM_END => self.hram.read_byte(address),
            INTERRUPTS_ENABLE => self.interrupts.interrupt_enable.bits(),
            _ => panic!("Reading memory that is out of bounds: 0x{:04X}", address),
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            ROM_BANK_00_START..=ROM_BANK_NN_END => self.cartridge.write_byte(address, value),
            VRAM_START..=VRAM_END => self.ppu.write_vram(address, value, &mut self.scheduler, &mut self.interrupts),
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => self.cartridge.write_external_ram(address, value),
            WRAM_BANK_00_START..=WRAM_BANK_00_END => self.wram.write_bank_0(address, value),
            WRAM_BANK_NN_START..=WRAM_BANK_NN_END => self.wram.write_bank_n(address, value),
            ECHO_RAM_START..=ECHO_RAM_END => self.wram.write_echo_ram(address, value),
            OAM_ATTRIBUTE_START..=OAM_ATTRIBUTE_END => self.ppu.write_vram(address, value, &mut self.scheduler, &mut self.interrupts),
            NOT_USABLE_START..=NOT_USABLE_END => log::trace!("ROM Writing to Non-usable memory: {:04X}", address),
            IO_START..=IO_END => self.write_io_byte(address, value),
            HRAM_START..=HRAM_END => self.hram.set_byte(address, value),
            INTERRUPTS_ENABLE => self.interrupts.overwrite_ie(value),
            _ => panic!("Writing to memory that is not in bounds: 0x{:04X}", address),
        }
    }

    /// Specific method for all calls to the IO registers.
    fn read_io_byte(&mut self, address: u16) -> u8 {
        match address {
            JOYPAD_REGISTER => self.joypad_register.get_register(),
            SIO_DATA => self.io_registers.read_byte(address),
            SIO_CONT => self.io_registers.read_byte(address),
            DIVIDER_REGISTER => self.timers.divider_register(&self.scheduler),
            TIMER_COUNTER..=TIMER_CONTROL => self.timers.read_register(address, &mut self.scheduler),
            INTERRUPTS_FLAG => self.interrupts.interrupt_flag.bits(),
            APU_MEM_START..=APU_MEM_END => self.apu.read_register(address, &mut self.scheduler, self.cgb_data.double_speed as u64),
            WAVE_SAMPLE_START..=WAVE_SAMPLE_END => self.apu.read_wave_sample(address, &mut self.scheduler, self.cgb_data.double_speed as u64),
            DMA_TRANSFER => self.io_registers.read_byte(address),
            CGB_PREPARE_SWITCH => {
                if self.emulated_model.is_cgb() {
                    self.cgb_data.read_prepare_switch()
                } else {
                    INVALID_READ
                }
            }
            0xFF4E => self.io_registers.read_byte(address),
            PPU_IO_START..=PPU_IO_END => self.ppu.read_vram(address),
            CGB_HDMA_1 | CGB_HDMA_2 | CGB_HDMA_3 | CGB_HDMA_4 => INVALID_READ,
            CGB_HDMA_5 => {
                if self.emulated_model.is_dmg() {
                    INVALID_READ
                } else {
                    self.hdma.hdma5()
                }
            }
            CGB_RP => self.io_registers.read_byte(address),
            PPU_CGB_IO_START..=PPU_CGB_IO_END => self.ppu.read_vram(address),
            CGB_WRAM_BANK => self.wram.read_bank_select(),
            _ => self.io_registers.read_byte(address),
        }
    }

    fn write_io_byte(&mut self, address: u16, value: u8) {
        // Temporary for BLARG's tests without visual aid, this writes to the Serial port
        if address == 0xFF02 && value == 0x81 {
            println!("Output: {}", self.read_byte(0xFF01) as char);
        }
        match address {
            JOYPAD_REGISTER => self.joypad_register.set_register(value),
            SIO_DATA => self.io_registers.write_byte(address, value),
            SIO_CONT => self.io_registers.write_byte(address, value),
            DIVIDER_REGISTER => self.timers.set_divider(&mut self.scheduler),
            TIMER_COUNTER..=TIMER_CONTROL => self.timers.write_register(address, value, &mut self.scheduler),
            INTERRUPTS_FLAG => self.interrupts.overwrite_if(value),
            APU_MEM_START..=APU_MEM_END => self.apu.write_register(
                address,
                value,
                &mut self.scheduler,
                self.emulated_model,
                self.cgb_data.double_speed as u64,
            ),
            WAVE_SAMPLE_START..=WAVE_SAMPLE_END => self.apu.write_wave_sample(address, value, &mut self.scheduler, self.cgb_data.double_speed as u64),
            DMA_TRANSFER => self.dma_transfer(value),
            CGB_PREPARE_SWITCH => self.cgb_data.write_prepare_switch(value),
            0xFF4E => self.io_registers.write_byte(address, value),
            PPU_IO_START..=PPU_IO_END => self.ppu.write_vram(address, value, &mut self.scheduler, &mut self.interrupts),
            CGB_HDMA_1 => self.hdma.write_hdma1(value),
            CGB_HDMA_2 => self.hdma.write_hdma2(value),
            CGB_HDMA_3 => self.hdma.write_hdma3(value),
            CGB_HDMA_4 => self.hdma.write_hdma4(value),
            CGB_HDMA_5 => {
                self.hdma.write_hdma5(value, &mut self.scheduler);
                // If a HDMA is started during HBlank one copy occurs right away.
                if self.ppu.get_current_mode() == Mode::Hblank {
                    self.hdma_check_and_transfer()
                }
            }
            0xFF50 if !self.boot_rom.is_finished => {
                self.boot_rom.is_finished = true;
                info!("Finished executing BootRom!");
            }
            CGB_RP => self.io_registers.write_byte(address, value),
            PPU_CGB_IO_START..=PPU_CGB_IO_END => self.ppu.write_vram(address, value, &mut self.scheduler, &mut self.interrupts),
            CGB_WRAM_BANK => self.wram.write_bank_select(value),
            _ => self.io_registers.write_byte(address, value),
        }
    }

    /// Simply returns 0xFF while also printing a warning to the logger.
    fn non_usable_call(&self, address: u16) -> u8 {
        warn!("ROM Accessed non usable memory: {:4X}", address);
        INVALID_READ
    }

    /// Synchronise all components to ensure we're in a consistent state for the
    /// current m-cycle.
    #[inline]
    fn synchronise_state_for_vblank(&mut self) {
        let speed_multiplier = self.get_speed_shift();
        self.apu.synchronise(&mut self.scheduler, speed_multiplier);
    }

    /// Ticks the scheduler by 4 cycles, executes any events if they come up.
    /// Returns true if a vblank interrupt happened.
    #[inline(always)]
    fn execute_scheduled_events(&mut self) -> bool {
        let mut vblank_occurred = false;

        while let Some(mut event) = self.scheduler.pop_closest() {
            match event.event_type {
                EventType::None => {
                    // On startup we should add OAM
                    self.scheduler.push_event(EventType::OamSearch, 0);
                    self.scheduler.push_event(EventType::TimerTick, self.timers.timer_control.get_clock_interval());
                }
                EventType::Vblank => {
                    self.ppu.vblank(&mut self.interrupts);
                    self.scheduler
                        .push_full_event(event.update_self(EventType::VblankWait, SCANLINE_DURATION << self.get_speed_shift()));
                    vblank_occurred = true;
                    // Used for APU syncing.
                    self.synchronise_state_for_vblank();
                }
                EventType::OamSearch => {
                    self.ppu.oam_search(&mut self.interrupts);
                    self.scheduler
                        .push_full_event(event.update_self(EventType::LcdTransfer, OAM_SEARCH_DURATION << self.get_speed_shift()));
                }
                EventType::LcdTransfer => {
                    self.ppu.lcd_transfer(&self.scheduler);
                    self.scheduler
                        .push_full_event(event.update_self(EventType::Hblank, self.ppu.get_lcd_transfer_duration() << self.get_speed_shift()));
                }
                EventType::Hblank => {
                    self.ppu.hblank(&mut self.interrupts);

                    // First 144 lines
                    if self.ppu.current_y != 143 {
                        self.scheduler
                            .push_full_event(event.update_self(EventType::OamSearch, self.ppu.get_hblank_duration() << self.get_speed_shift()));
                    } else {
                        self.scheduler
                            .push_full_event(event.update_self(EventType::Vblank, self.ppu.get_hblank_duration() << self.get_speed_shift()));
                    }

                    // HDMA transfers 16 bytes every HBLANK
                    self.hdma_check_and_transfer();
                }
                EventType::VblankWait => {
                    self.ppu.vblank_wait(&mut self.interrupts);

                    if self.ppu.current_y != 153 {
                        self.scheduler
                            .push_full_event(event.update_self(EventType::VblankWait, SCANLINE_DURATION << self.get_speed_shift()));
                    } else {
                        self.scheduler
                            .push_full_event(event.update_self(EventType::OamSearch, SCANLINE_DURATION << self.get_speed_shift()));
                        self.scheduler.push_relative(EventType::Y153TickToZero, 4);
                    }
                }
                EventType::TimerOverflow => {
                    self.timers.timer_overflow(&mut self.scheduler, &mut self.interrupts);
                }
                EventType::TimerPostOverflow => {
                    self.timers.just_overflowed = false;
                }
                EventType::TimerTick => self.timers.scheduled_timer_tick(&mut self.scheduler),
                EventType::DMARequested => {
                    let address = (self.io_registers.read_byte(DMA_TRANSFER) as usize) << 8;
                    let shadow_oam = self.gather_shadow_oam(address);
                    self.ppu.oam_dma_transfer(&shadow_oam, &mut self.scheduler);
                }
                EventType::DMATransferComplete => {
                    self.ppu.oam_dma_finished();
                }
                EventType::GDMARequested => {
                    log::info!("Performing GDMA transfer at cycle: {}", self.scheduler.current_time);
                    let mut clocks_to_wait =
                        (self.hdma.transfer_size / 16) as u64 * if self.cgb_data.double_speed { 64 } else { 32 };
                    self.scheduler.push_relative(EventType::GDMATransferComplete, clocks_to_wait);
                    self.gdma_transfer();
                    while clocks_to_wait > 0 {
                        if self.do_m_cycle() {
                            vblank_occurred = true;
                        }
                        clocks_to_wait -= 4;
                    }
                }
                EventType::GDMATransferComplete => {
                    // If a new transfer is started without updating these registers they should
                    // continue where they left off.
                    log::info!(
                        "Completing GDMA transfer at clock cycle: {}",
                        self.scheduler.current_time
                    );
                    self.hdma.source_address += self.hdma.transfer_size;
                    self.hdma.destination_address += self.hdma.transfer_size;

                    self.hdma.complete_transfer();
                }
                EventType::Y153TickToZero => {
                    self.ppu.late_y_153_to_0(&mut self.interrupts);
                }
            };
        }
        vblank_occurred
    }

    /// Add a new interrupt to the IF flag.
    #[inline]
    pub fn add_new_interrupts(&mut self, interrupt: Option<InterruptFlags>) {
        if let Some(intr) = interrupt {
            self.interrupts.insert_interrupt(intr);
        }
    }

    pub fn get_speed_shift(&self) -> u64 {
        self.cgb_data.double_speed as u64
    }
}

impl MemoryMapper for Memory {
    fn read_byte(&mut self, address: u16) -> u8 {
        self.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.write_byte(address, value)
    }

    fn boot_rom_finished(&self) -> bool {
        self.boot_rom.is_finished
    }

    fn get_mode(&self) -> GameBoyModel {
        self.emulated_model
    }

    fn cartridge(&self) -> Option<&Cartridge> {
        Some(&self.cartridge)
    }

    fn interrupts(&self) -> &Interrupts {
        &self.interrupts
    }

    fn interrupts_mut(&mut self) -> &mut Interrupts {
        &mut self.interrupts
    }

    fn turn_on_lcd(&mut self) {
        self.ppu.turn_on_lcd(&mut self.scheduler, &mut self.interrupts);
    }

    fn turn_off_lcd(&mut self) {
        self.ppu.turn_off_lcd(&mut self.scheduler);
    }

    fn cgb_data(&mut self) -> &mut CgbSpeedData {
        &mut self.cgb_data
    }

    fn do_m_cycle(&mut self) -> bool {
        self.scheduler.add_cycles(4);
        self.execute_scheduled_events()
    }

    fn execute_next_event(&mut self) -> bool {
        self.scheduler.skip_to_next_event();
        self.execute_scheduled_events()
    }
}

impl Debug for Memory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Memory: {:?}\nCartridge: {:?}", self.io_registers, self.cartridge)
    }
}
