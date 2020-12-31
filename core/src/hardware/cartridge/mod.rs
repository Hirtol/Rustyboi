use std::fmt;

use bitflags::_core::fmt::{Debug, Formatter};

use crate::hardware::cartridge::header::CartridgeHeader;
use crate::hardware::cartridge::mbc::{MBC1State, MBC3State, MBC5State, MBC, ROM_BANK_SIZE};
use crate::hardware::mmu::INVALID_READ;

pub mod header;
pub mod mbc;

pub struct Cartridge {
    header: CartridgeHeader,
    has_battery: bool,
    lower_bank_offset: usize,
    higher_bank_offset: usize,
    effective_rom_banks: usize,
    ram_offset: usize,
    rom: Vec<u8>,
    ram: Vec<u8>,
    mbc: MBC,
}

impl Cartridge {
    pub fn new(rom: &[u8], saved_ram: Option<Vec<u8>>) -> Self {
        let header = CartridgeHeader::new(rom);
        let (mbc, has_battery) = create_mbc(&header);
        let mut ex_ram = vec![INVALID_READ; header.ram_size.to_usize()];

        if let Some(mut ram) = saved_ram {
            if ram.len() < header.ram_size.to_usize() {
                ram.extend_from_slice(&vec![INVALID_READ; header.ram_size.to_usize() - ram.len()]);
            }
            ex_ram = ram;
        }

        log::info!("Loading ROM with header: {:#X?}", header);

        Cartridge {
            header,
            has_battery,
            lower_bank_offset: 0,
            higher_bank_offset: 0x4000,
            ram_offset: 0,
            effective_rom_banks: rom.len() / ROM_BANK_SIZE,
            rom: rom.to_vec(),
            ram: ex_ram,
            mbc,
        }
    }

    pub fn read_0000_3fff(&self, address: u16) -> u8 {
        self.rom[(address & 0x3FFF) as usize | self.lower_bank_offset]
    }

    pub fn read_4000_7fff(&self, address: u16) -> u8 {
        // first 14 bits of the address, and then the rom bank shifted onto it.
        self.rom[(address & 0x3FFF) as usize | self.higher_bank_offset]
    }

    pub fn read_external_ram(&self, address: u16) -> u8 {
        let address = (address & 0x1FFF) as usize;
        match &self.mbc {
            MBC::MBC0 if self.ram.len() > 0 => self.ram[address],
            MBC::MBC1(state) if state.ram_enabled => self.ram[address | self.ram_offset],
            MBC::MBC3(state) if state.ram_enabled => match state.ram_bank {
                0x0..=0x7 => self.ram[address + self.ram_offset],
                0x8..=0xC => state.read_rtc_register(),
                _ => unreachable!(),
            },
            MBC::MBC5(state) if state.ram_enabled => self.ram[address + self.ram_offset],
            _ => INVALID_READ,
        }
    }

    pub fn write_external_ram(&mut self, address: u16, value: u8) {
        let address = (address & 0x1FFF) as usize;
        match &mut self.mbc {
            MBC::MBC0 if self.ram.len() > 0 => {
                self.ram[address] = value;
            }
            MBC::MBC1(state) if state.ram_enabled => {
                self.ram[address | self.ram_offset] = value;
            }
            MBC::MBC3(state) if state.ram_enabled => match state.ram_bank {
                0x0..=0x7 => self.ram[address + self.ram_offset] = value,
                0x8..=0xC => state.write_rtc_register(value),
                _ => unreachable!(),
            },
            MBC::MBC5(state) if state.ram_enabled => {
                self.ram[address + self.ram_offset] = value;
            }
            _ => {}
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        match &mut self.mbc {
            MBC::MBC0 => {}
            MBC::MBC1(state) => match address {
                0x0000..=0x1FFF => state.enable_ram(value),
                0x2000..=0x3FFF => {
                    state.set_lower_rom_bank(value, self.effective_rom_banks);
                    self.higher_bank_offset = state.get_7fff_offset();
                }
                0x4000..=0x5FFF => {
                    state.set_higher_rom_bank(value, self.effective_rom_banks);
                    self.lower_bank_offset = state.get_3fff_offset(self.effective_rom_banks);
                    self.higher_bank_offset = state.get_7fff_offset();
                    self.ram_offset = state.get_ram_offset(self.ram.len());
                }
                0x6000..=0x7FFF => {
                    state.set_bank_mode_select(value);
                    self.lower_bank_offset = state.get_3fff_offset(self.effective_rom_banks);
                    self.ram_offset = state.get_ram_offset(self.ram.len());
                }
                _ => {}
            },
            MBC::MBC3(state) => match address {
                0x0000..=0x1FFF => state.enable_ram(value),
                0x2000..=0x3FFF => {
                    state.write_lower_rom_bank(value, self.effective_rom_banks);
                    self.higher_bank_offset = state.get_7fff_offset();
                }
                0x4000..=0x5FFF => {
                    state.write_ram_bank(value);
                    self.ram_offset = state.get_ram_offset();
                }
                0x6000..=0x7FFF => {
                    state.write_latch_data(value);
                }
                _ => {}
            },
            MBC::MBC5(state) => match address {
                0x0000..=0x1FFF => state.enable_ram(value),
                0x2000..=0x2FFF => {
                    state.write_lower_rom_bank(value, self.effective_rom_banks);
                    self.higher_bank_offset = state.get_7fff_offset();
                }
                0x3000..=0x3FFF => {
                    state.write_higher_rom_bank(value, self.effective_rom_banks);
                    self.higher_bank_offset = state.get_7fff_offset();
                }
                0x4000..=0x5FFF => {
                    state.write_ram_bank(value);
                    self.ram_offset = state.get_ram_offset();
                }
                _ => {}
            },
        }
    }

    pub fn cartridge_header(&self) -> &CartridgeHeader {
        &self.header
    }

    /// Retrieves the current battery ram state.
    /// Ideally this would be done via an MMAP so that the battery ram is always saved,
    /// even in the case of an emulator crash.
    pub fn battery_ram(&self) -> Option<&[u8]> {
        if self.has_battery {
            Some(&self.ram)
        } else {
            None
        }
    }
}

impl Debug for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cartridge {{ header: {:?} }}", self.header)
    }
}

fn create_mbc(header: &CartridgeHeader) -> (MBC, bool) {
    use MBC::*;
    let has_battery = match header.cartridge_type as u8 {
        0x3 | 0x6 | 0x9 | 0xD | 0xF | 0x10 | 0x13 | 0x1B | 0x1E | 0x22 | 0xFF => true,
        _ => false,
    };
    let mbc = match header.cartridge_type as u8 {
        0x0 => MBC0,
        0x1..=0x3 => MBC1(MBC1State::default()),
        0xF..=0x13 => MBC3(MBC3State::default()),
        // 1C..=1E technically contain a rumble feature, to be implemented.
        0x19..=0x1E => MBC5(MBC5State::default()),
        _ => panic!(
            "Unsupported cartridge type, please add support for: {:#?}",
            header.cartridge_type
        ),
    };

    (mbc, has_battery)
}
