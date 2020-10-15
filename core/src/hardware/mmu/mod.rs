use std::fmt;

use bitflags::_core::fmt::{Debug, Formatter};
use bitflags::_core::ops::{Add, Mul, Sub};
use itertools::Itertools;
use log::*;

use hram::Hram;

use crate::emulator::EmulatorMode;
use crate::emulator::EmulatorMode::DMG;
use crate::EmulatorOptions;
use crate::hardware::apu::{
    APU, APU_MEM_END, APU_MEM_START, FRAME_SEQUENCE_CYCLES, SAMPLE_CYCLES, WAVE_SAMPLE_END, WAVE_SAMPLE_START,
};
use crate::hardware::cartridge::Cartridge;
use crate::hardware::mmu::cgb_mem::{CgbData, HdmaRegister};
use crate::hardware::mmu::cgb_mem::HdmaMode::HDMA;
use crate::hardware::mmu::wram::Wram;
use crate::hardware::ppu::{DMA_TRANSFER, PPU};
use crate::hardware::ppu::tiledata::*;
use crate::io::bootrom::BootRom;
use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::io::io_registers::*;
use crate::io::joypad::*;
use crate::io::timer::*;
use crate::scheduler::{Event, EventType, Scheduler};
use crate::scheduler::EventType::{DMARequested, DMATransferComplete};

pub mod cgb_mem;
mod hram;
mod wram;

pub const MEMORY_SIZE: usize = 0x10000;
/// 16 KB ROM bank, usually 00. From Cartridge, read-only
pub const ROM_BANK_00_START: u16 = 0x0000;
pub const ROM_BANK_00_END: u16 = 0x03FFF;
/// 16 KB Rom Bank 01~NN. From cartridge, switchable bank via Memory Bank. Read-only.
pub const ROM_BANK_NN_START: u16 = 0x4000;
pub const ROM_BANK_NN_END: u16 = 0x7FFF;
/// This area contains information about the program,
/// its entry point, checksums, information about the used MBC chip, the ROM and RAM sizes, etc.
pub const CARTRIDGE_HEADER_START: u16 = 0x0100;
pub const CARTRIDGE_HEADER_END: u16 = 0x014F;
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
/// The amount the ECHO_RAM_ADDRESS needs to have subtracted to get to the corresponding WRAM.
pub const ECHO_RAM_OFFSET: u16 = 0x2000;
/// Sprite attribute table (OAM)
pub const OAM_ATTRIBUTE_START: u16 = 0xFE00;
pub const OAM_ATTRIBUTE_END: u16 = 0xFE9F;
/// Not usable
pub const NOT_USABLE_START: u16 = 0xFEA0;
pub const NOT_USABLE_END: u16 = 0xFEFF;
/// I/O Registers
pub const IO_START: u16 = 0xFF00;
pub const IO_END: u16 = 0xFF7F;

pub const CGB_PREPARE_SWITCH: u16 = 0xFF4D;
pub const CGB_VRAM_BANK_REGISTER: u16 = 0xFF4F;
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
///This register serves as a flag for which object priority mode to use. While the DMG prioritizes
///objects by x-coordinate, the CGB prioritizes them by location in OAM.
/// This flag is set by the CGB bios after checking the game's CGB compatibility.
pub const CGB_OBJECT_PRIORITY_MODE: u16 = 0xFF6C;
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
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);
    fn boot_rom_finished(&self) -> bool;
    fn get_mode(&self) -> EmulatorMode;
    /// Returns, if the current ROM has a battery, the contents of the External Ram.
    ///
    /// Should be used for saving functionality.
    fn cartridge(&self) -> Option<&Cartridge>;
    fn interrupts(&self) -> &Interrupts;
    fn interrupts_mut(&mut self) -> &mut Interrupts;
    fn turn_on_lcd(&mut self);
    fn turn_off_lcd(&mut self);
    fn cgb_data(&mut self) -> &mut CgbData;
    /// Perform one M-cycle (4 cycles) on all components of the system.
    /// Returns `true` if V-blank occurred
    fn do_m_cycle(&mut self) -> bool;
}

pub struct Memory {
    boot_rom: BootRom,
    cartridge: Cartridge,
    pub scheduler: Scheduler,
    pub emulation_mode: EmulatorMode,
    pub cgb_data: CgbData,
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
    pub fn new(cartridge: &[u8], emu_opts: EmulatorOptions) -> Self {
        let mut result = Memory {
            boot_rom: BootRom::new(emu_opts.boot_rom),
            cartridge: Cartridge::new(cartridge, emu_opts.saved_ram),
            scheduler: Scheduler::new(),
            emulation_mode: emu_opts.emulator_mode,
            cgb_data: CgbData::new(),
            hdma: HdmaRegister::new(),
            ppu: PPU::new(emu_opts.display_colour),
            apu: APU::new(),
            hram: Hram::new(),
            wram: Wram::new(),
            joypad_register: JoyPad::new(),
            timers: Default::default(),
            interrupts: Default::default(),
            io_registers: IORegisters::new(),
        };

        // If we're not doing the CGB bootrom AND the cartridge is not a CGB, we swich to DMG
        if !result.cartridge.cartridge_header().cgb_flag && result.boot_rom_finished() {
            result.emulation_mode = DMG;
        }

        result
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x00FF if !self.boot_rom.is_finished => self.boot_rom.read_byte(address),
            0x0200..=0x08FF if !self.boot_rom.is_finished && self.emulation_mode.is_cgb() => self.boot_rom.read_byte(address),
            ROM_BANK_00_START..=ROM_BANK_00_END => self.cartridge.read_0000_3fff(address),
            ROM_BANK_NN_START..=ROM_BANK_NN_END => self.cartridge.read_4000_7fff(address),
            TILE_BLOCK_0_START..=TILE_BLOCK_2_END => self.ppu.get_tile_byte(address),
            TILEMAP_9800_START..=TILEMAP_9C00_END => self.ppu.get_tilemap_byte(address),
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => self.cartridge.read_external_ram(address),
            WRAM_BANK_00_START..=WRAM_BANK_00_END => self.wram.read_bank_0(address),
            WRAM_BANK_NN_START..=WRAM_BANK_NN_END => self.wram.read_bank_n(address),
            ECHO_RAM_START..=ECHO_RAM_END => self.wram.read_echo_ram(address),
            OAM_ATTRIBUTE_START..=OAM_ATTRIBUTE_END => self.ppu.get_oam_byte(address),
            NOT_USABLE_START..=NOT_USABLE_END => self.non_usable_call(address),
            IO_START..=IO_END => self.read_io_byte(address),
            HRAM_START..=HRAM_END => self.hram.read_byte(address),
            INTERRUPTS_ENABLE => {
                //log::info!("Reading interrupt enable {:?}", self.interrupts_enable);
                self.interrupts.interrupt_enable.bits()
            }
            _ => panic!("Reading memory that is out of bounds: 0x{:04X}", address),
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            ROM_BANK_00_START..=ROM_BANK_NN_END => self.cartridge.write_byte(address, value),
            TILE_BLOCK_0_START..=TILE_BLOCK_2_END => self.ppu.set_tile_byte(address, value),
            TILEMAP_9800_START..=TILEMAP_9C00_END => self.ppu.set_tilemap_byte(address, value),
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => self.cartridge.write_byte(address, value),
            WRAM_BANK_00_START..=WRAM_BANK_00_END => self.wram.write_bank_0(address, value),
            WRAM_BANK_NN_START..=WRAM_BANK_NN_END => self.wram.write_bank_n(address, value),
            ECHO_RAM_START..=ECHO_RAM_END => self.wram.write_echo_ram(address, value),
            OAM_ATTRIBUTE_START..=OAM_ATTRIBUTE_END => self.ppu.set_oam_byte(address, value),
            NOT_USABLE_START..=NOT_USABLE_END => log::trace!("ROM Writing to Non-usable memory: {:04X}", address),
            IO_START..=IO_END => self.write_io_byte(address, value),
            HRAM_START..=HRAM_END => self.hram.set_byte(address, value),
            INTERRUPTS_ENABLE => {
                //log::info!("Writing Interrupt Enable: {:?}", InterruptFlags::from_bits_truncate(value));
                self.interrupts.overwrite_ie(value);
            }
            _ => panic!("Writing to memory that is not in bounds: 0x{:04X}", address),
        }
    }

    /// Specific method for all calls to the IO registers.
    fn read_io_byte(&self, address: u16) -> u8 {
        use crate::hardware::ppu::*;
        match address {
            JOYPAD_REGISTER => self.joypad_register.get_register(),
            SIO_DATA => self.io_registers.read_byte(address),
            SIO_CONT => self.io_registers.read_byte(address),
            DIVIDER_REGISTER => self.timers.divider_register(),
            TIMER_COUNTER => self.timers.timer_counter,
            TIMER_MODULO => self.timers.timer_modulo,
            TIMER_CONTROL => self.timers.timer_control.to_bits(),
            INTERRUPTS_FLAG => {
                //log::info!("Reading interrupt flag {:?}", self.interrupts.interrupt_flag);
                self.interrupts.interrupt_flag.bits()
            }
            APU_MEM_START..=APU_MEM_END => {
                let result = self.apu.read_register(address);
                //log::info!("APU Read on address: 0x{:02X} with return value: 0x{:02X}", address, result);
                result
            }
            WAVE_SAMPLE_START..=WAVE_SAMPLE_END => {
                let result = self.apu.read_wave_sample(address);
                //log::info!("APU Wave_Read on address: 0x{:02X} with return value: 0x{:02X}", address, result);
                result
            }
            LCD_CONTROL_REGISTER => self.ppu.get_lcd_control(),
            LCD_STATUS_REGISTER => self.ppu.get_lcd_status(),
            SCY_REGISTER => self.ppu.get_scy(),
            SCX_REGISTER => self.ppu.get_scx(),
            LY_REGISTER => self.ppu.get_ly(),
            LYC_REGISTER => self.ppu.get_lyc(),
            DMA_TRANSFER => self.io_registers.read_byte(address),
            BG_PALETTE => self.ppu.get_bg_palette(),
            OB_PALETTE_0 => self.ppu.get_oam_palette_0(),
            OB_PALETTE_1 => self.ppu.get_oam_palette_1(),
            WY_REGISTER => self.ppu.get_window_y(),
            WX_REGISTER => self.ppu.get_window_x(),
            CGB_PREPARE_SWITCH => if self.emulation_mode.is_cgb() {
                self.cgb_data.read_prepare_switch()
            } else {
                0xFF
            },
            CGB_VRAM_BANK_REGISTER => self.ppu.get_vram_bank(),
            CGB_HDMA_1 | CGB_HDMA_2 | CGB_HDMA_3 | CGB_HDMA_4 => INVALID_READ,
            CGB_HDMA_5 => if self.emulation_mode.is_dmg() { INVALID_READ } else { self.hdma.hdma5() }
            CGB_RP => self.io_registers.read_byte(address),
            CGB_BACKGROUND_COLOR_INDEX => self.ppu.get_bg_color_palette_index(),
            CGB_BACKGROUND_PALETTE_DATA => self.ppu.get_bg_palette_data(),
            CGB_SPRITE_COLOR_INDEX => self.ppu.get_sprite_color_palette_index(),
            CGB_OBJECT_PALETTE_DATA => self.ppu.get_obj_palette_data(),
            CGB_OBJECT_PRIORITY_MODE => self.ppu.get_object_priority(),
            CGB_WRAM_BANK => self.wram.read_bank_select(),
            _ => self.io_registers.read_byte(address),
        }
    }

    fn write_io_byte(&mut self, address: u16, value: u8) {
        use crate::hardware::ppu::*;
        // Temporary for BLARG's tests without visual aid, this writes to the Serial port
        if address == 0xFF02 && value == 0x81 {
            println!("Output: {}", self.read_byte(0xFF01) as char);
        }
        match address {
            JOYPAD_REGISTER => self.joypad_register.set_register(value),
            SIO_DATA => self.io_registers.write_byte(address, value),
            SIO_CONT => self.io_registers.write_byte(address, value),
            DIVIDER_REGISTER => self.timers.set_divider(&mut self.scheduler),
            TIMER_COUNTER => self.timers.set_timer_counter(value, &mut self.scheduler),
            TIMER_MODULO => self.timers.set_tma(value),
            TIMER_CONTROL => self.timers.set_timer_control(value, &mut self.scheduler),
            INTERRUPTS_FLAG => self.interrupts.overwrite_if(value),
            APU_MEM_START..=APU_MEM_END => self.apu.write_register(address, value, &mut self.scheduler),
            WAVE_SAMPLE_START..=WAVE_SAMPLE_END => self.apu.write_wave_sample(address, value),
            LCD_CONTROL_REGISTER => self.ppu.set_lcd_control(value, &mut self.scheduler, &mut self.interrupts),
            LCD_STATUS_REGISTER => self.ppu.set_lcd_status(value, &mut self.interrupts),
            SCY_REGISTER => self.ppu.set_scy(value),
            SCX_REGISTER => self.ppu.set_scx(value),
            LY_REGISTER => self.ppu.set_ly(value),
            LYC_REGISTER => self.ppu.set_lyc(value, &mut self.interrupts),
            DMA_TRANSFER => self.dma_transfer(value),
            BG_PALETTE => self.ppu.set_bg_palette(value),
            OB_PALETTE_0 => self.ppu.set_oam_palette_0(value),
            OB_PALETTE_1 => self.ppu.set_oam_palette_1(value),
            WY_REGISTER => self.ppu.set_window_y(value),
            WX_REGISTER => self.ppu.set_window_x(value),
            CGB_PREPARE_SWITCH => self.cgb_data.write_prepare_switch(value),
            CGB_VRAM_BANK_REGISTER => self.ppu.set_vram_bank(value),
            CGB_HDMA_1 => self.hdma.write_hdma1(value),
            CGB_HDMA_2 => self.hdma.write_hdma2(value),
            CGB_HDMA_3 => self.hdma.write_hdma3(value),
            CGB_HDMA_4 => self.hdma.write_hdma4(value),
            CGB_HDMA_5 => self.hdma.write_hdma5(value, &mut self.scheduler),
            0xFF50 if !self.boot_rom.is_finished => {
                self.boot_rom.is_finished = true;
                // If the cartridge doesn't support CGB at all we switch to DMG mode.
                if !self.cartridge.cartridge_header().cgb_flag {
                    self.emulation_mode = EmulatorMode::DMG;
                }
                info!("Finished executing BootRom!");
            }
            CGB_RP => self.io_registers.write_byte(address, value),
            CGB_BACKGROUND_COLOR_INDEX => self.ppu.set_bg_color_palette_index(value),
            CGB_BACKGROUND_PALETTE_DATA => self.ppu.set_bg_palette_data(value),
            CGB_SPRITE_COLOR_INDEX => self.ppu.set_sprite_color_palette_index(value),
            CGB_OBJECT_PALETTE_DATA => self.ppu.set_obj_palette_data(value),
            CGB_OBJECT_PRIORITY_MODE => self.ppu.set_object_priority(value),
            CGB_WRAM_BANK => self.wram.write_bank_select(value),
            _ => self.io_registers.write_byte(address, value)
        }
    }

    /// Starts the sequence of events for a OAM DMA transfer.
    fn dma_transfer(&mut self, value: u8) {
        self.io_registers.write_byte(DMA_TRANSFER, value);
        // In case a previous DMA was running we should cancel it.
        self.scheduler.remove_event_type(DMATransferComplete);
        // 4 Cycles after the request is when the DMA is actually started.
        self.scheduler.push_relative(DMARequested, 4);
    }

    fn gather_shadow_oam(&self, start_address: usize) -> Vec<u8> {
        (0..0xA0).map(|i| self.read_byte((start_address + i) as u16)).collect()
    }

    fn gather_gdma_data(&self) -> Vec<u8> {
        (self.hdma.source_address..(self.hdma.source_address + self.hdma.transfer_size)).map(|i| self.read_byte(i)).collect()
    }

    /// Simply returns 0xFF while also printing a warning to the logger.
    fn non_usable_call(&self, address: u16) -> u8 {
        warn!("ROM Accessed non usable memory: {:4X}", address);
        INVALID_READ
    }

    /// Ticks the scheduler by 4 cycles, executes any events if they come up.
    /// Returns true if a vblank interrupt happened.
    fn tick_scheduler(&mut self) -> bool {
        let mut vblank_occurred = false;
        self.scheduler.add_cycles(4);

        while let Some(mut event) = self.scheduler.pop_closest() {
            match event.event_type {
                EventType::NONE => {
                    // On startup we should add OAM
                    self.scheduler
                        .push_full_event(event.update_self(EventType::OamSearch, 0));
                    self.scheduler.push_event(EventType::APUFrameSequencer, 8192);
                    self.scheduler.push_event(EventType::APUSample, 95);
                }
                EventType::VBLANK => {
                    self.ppu.vblank(&mut self.interrupts);
                    self.scheduler
                        .push_full_event(event.update_self(EventType::VblankWait, 456 << self.get_speed_shift()));
                    vblank_occurred = true;
                }
                EventType::OamSearch => {
                    self.ppu.oam_search(&mut self.interrupts);
                    self.scheduler
                        .push_full_event(event.update_self(EventType::LcdTransfer, 80 << self.get_speed_shift()));
                }
                EventType::LcdTransfer => {
                    self.ppu.lcd_transfer(self.emulation_mode);
                    self.scheduler
                        .push_full_event(event.update_self(EventType::HBLANK, 172 << self.get_speed_shift()));
                }
                EventType::HBLANK => {
                    self.ppu.hblank(&mut self.interrupts);

                    // First 144 lines
                    if self.ppu.current_y != 143 {
                        self.scheduler
                            .push_full_event(event.update_self(EventType::OamSearch, 204 << self.get_speed_shift()));
                    } else {
                        self.scheduler
                            .push_full_event(event.update_self(EventType::VBLANK, 204 << self.get_speed_shift()));
                    }

                    // HDMA transfers 16 bytes every HBLANK
                    // TODO: move this to own scheduler, since this costs ~200 fps having this here.
                    if self.hdma.transfer_ongoing && self.hdma.current_mode == HDMA {
                        log::info!("Performing HDMA transfer");
                        self.hdma_transfer();
                        if self.hdma.transfer_ongoing {
                            self.do_m_cycle();
                            // Pass 36 (single speed)/68 (double speed) cycles where the CPU does nothing.
                            for _ in 0..(8 << self.get_speed_shift()) {
                                //TODO: Skip ahead, since CPU is halted during transfer. Account for double speed
                                self.do_m_cycle();
                            }
                        }
                    }
                }
                EventType::VblankWait => {
                    self.ppu.vblank_wait(&mut self.interrupts);

                    if self.ppu.current_y != 0 {
                        self.scheduler
                            .push_full_event(event.update_self(EventType::VblankWait, 456 << self.get_speed_shift()));
                    } else {
                        self.scheduler
                            .push_full_event(event.update_self(EventType::OamSearch, 0));
                    }
                }
                EventType::APUFrameSequencer => {
                    // Both APU events rely on the scheduler being called after the APU tick
                    // (at least, that's how I used to do it in the APU tick function)
                    // Be careful about reordering.
                    self.apu.tick_frame_sequencer();
                    self.scheduler
                        .push_full_event(event.update_self(EventType::APUFrameSequencer, FRAME_SEQUENCE_CYCLES << self.get_speed_shift()));
                }
                EventType::APUSample => {
                    self.apu.tick_sampling_handler();
                    self.scheduler
                        .push_full_event(event.update_self(EventType::APUSample, SAMPLE_CYCLES << self.get_speed_shift()));
                }
                EventType::TimerOverflow => {
                    self.timers.timer_overflow(&mut self.scheduler, &mut self.interrupts);
                }
                EventType::TimerPostOverflow => {
                    self.timers.just_overflowed = false;
                }
                EventType::DMARequested => {
                    let address = (self.io_registers.read_byte(DMA_TRANSFER) as usize) << 8;
                    self.ppu.oam_dma_transfer(&self.gather_shadow_oam(address), &mut self.scheduler);
                }
                EventType::DMATransferComplete => {
                    self.ppu.oam_dma_finished();
                }
                EventType::GDMARequested => {
                    log::info!("Performing GDMA transfer at cycle: {}", self.scheduler.current_time);
                    let mut clocks_to_wait = (self.hdma.transfer_size / 16) as u64 * if self.cgb_data.double_speed { 64 } else { 32 };
                    self.scheduler.push_relative(EventType::GDMATransferComplete, clocks_to_wait);
                    self.gdma_transfer();

                    while clocks_to_wait > 0 {
                        self.do_m_cycle();
                        clocks_to_wait -= 4;
                    }
                }
                EventType::GDMATransferComplete => {
                    // If a new transfer is started without updating these registers they should
                    // continue where they left off.
                    log::warn!("Completing transfer at clock cycle: {}", self.scheduler.current_time);
                    self.hdma.source_address += self.hdma.transfer_size;
                    self.hdma.destination_address += self.hdma.transfer_size;

                    self.hdma.complete_transfer();
                }
            };
        }
        vblank_occurred
    }

    /// Required here since the GDMA can write to arbitrary PPU addresses.
    pub fn gdma_transfer(&mut self) {
        log::info!("Performing GDMA from source: [{:#4X}, {:#4X}] to destination: {:#4X}", self.hdma.source_address, self.hdma.source_address+self.hdma.transfer_size, self.hdma.destination_address);
        let values_iter = (self.hdma.source_address..(self.hdma.source_address + self.hdma.transfer_size))
            .map(|i| self.read_byte(i)).collect_vec();

        for (i, value) in values_iter.into_iter().enumerate() {
            self.write_byte(self.hdma.destination_address + i as u16, value);
        }
    }

    /// Required here since the HDMA can write to arbitrary PPU addresses.
    pub fn hdma_transfer(&mut self) {
        // We transfer 16 bytes every H-Blank
        let values_iter = (self.hdma.source_address..(self.hdma.source_address + 16))
            .map(|i| self.read_byte(i)).collect_vec();

        for (i, value) in values_iter.into_iter().enumerate() {
            self.write_byte(self.hdma.destination_address + i as u16, value);
        }

        self.hdma.advance_hdma();
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
    fn read_byte(&self, address: u16) -> u8 {
        self.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.write_byte(address, value)
    }

    fn boot_rom_finished(&self) -> bool {
        self.boot_rom.is_finished
    }

    fn get_mode(&self) -> EmulatorMode {
        self.emulation_mode
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

    fn cgb_data(&mut self) -> &mut CgbData {
        &mut self.cgb_data
    }

    fn do_m_cycle(&mut self) -> bool {
        //TODO: What if this is called by HDMA/GDMA?
        if self.cgb_data.double_speed {
            self.apu.tick(2);
        } else {
            self.apu.tick(4);
        }


        let result = self.tick_scheduler();
        // Timer has to be ticked after the Scheduler to make timings work out for MoonEye tests.
        self.timers.tick_timers(&mut self.scheduler);

        result
    }

    fn turn_on_lcd(&mut self) {
        self.ppu.turn_on_lcd(&mut self.scheduler, &mut self.interrupts);
    }

    fn turn_off_lcd(&mut self) {
        self.ppu.turn_off_lcd(&mut self.scheduler);
    }
}

impl Debug for Memory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Memory: {:?}\nCartridge: {:?}", self.io_registers, self.cartridge)
    }
}
