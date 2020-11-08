use crate::hardware::cartridge::header::CartridgeHeader;
use crate::hardware::cartridge::mbc::{MBC, MBC0, MBC1, MBC5};
use crate::hardware::cartridge::mbc3::MBC3;
use bitflags::_core::fmt::{Debug, Formatter};
use std::fmt;

pub mod header;
pub mod mbc;
mod mbc3;

pub struct Cartridge {
    header: CartridgeHeader,
    mbc: Box<dyn MBC + Send>,
}

impl Cartridge {
    pub fn new(rom: &[u8], saved_ram: Option<Vec<u8>>) -> Self {
        let header = CartridgeHeader::new(rom);
        log::debug!("Loading ROM with type: {:#X}, CGB-flag: {}", header.cartridge_type, header.cgb_flag);
        let mbc = create_mbc(&header, rom, saved_ram);
        Cartridge { header, mbc }
    }

    pub fn read_0000_3fff(&self, address: u16) -> u8 {
        self.mbc.read_3fff(address)
    }

    pub fn read_4000_7fff(&self, address: u16) -> u8 {
        self.mbc.read_7fff(address)
    }

    pub fn read_external_ram(&self, address: u16) -> u8 {
        self.mbc.read_ex_ram(address)
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        self.mbc.write_byte(address, value);
    }

    pub fn cartridge_header(&self) -> &CartridgeHeader {
        &self.header
    }

    pub fn mbc(&self) -> &dyn MBC {
        self.mbc.as_ref()
    }
}

impl Debug for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cartridge {{ header: {:?} }}", self.header)
    }
}

fn create_mbc(header: &CartridgeHeader, rom: &[u8], saved_ram: Option<Vec<u8>>) -> Box<dyn MBC + Send> {
    let rom_vec = rom.to_vec();

    match header.cartridge_type {
        0x0 => Box::new(MBC0::new(rom_vec)),
        0x1 => Box::new(MBC1::new(rom_vec, false, &header.ram_size, None)),
        // Potentially need to specify RAM + Battery for MBC1.
        0x2 => Box::new(MBC1::new(rom_vec, false, &header.ram_size, None)),
        0x3 => Box::new(MBC1::new(rom_vec, true, &header.ram_size, saved_ram)),
        0xF => Box::new(MBC3::new(rom_vec, true, &header.ram_size, None)),
        0x10 => Box::new(MBC3::new(rom_vec, true, &header.ram_size, saved_ram)),
        0x11 => Box::new(MBC3::new(rom_vec, false, &header.ram_size, None)),
        0x12 => Box::new(MBC3::new(rom_vec, false, &header.ram_size, saved_ram)),
        0x13 => Box::new(MBC3::new(rom_vec, true, &header.ram_size, saved_ram)),
        0x19 => Box::new(MBC5::new(rom_vec, false, &header.ram_size, None)),
        0x1A => Box::new(MBC5::new(rom_vec, false, &header.ram_size, None)),
        0x1B => Box::new(MBC5::new(rom_vec, true, &header.ram_size, saved_ram)),
        // These three technically contain a rumble feature, to be implemented.
        0x1C => Box::new(MBC5::new(rom_vec, false, &header.ram_size, None)),
        0x1D => Box::new(MBC5::new(rom_vec, false, &header.ram_size, None)),
        0x1E => Box::new(MBC5::new(rom_vec, true, &header.ram_size, saved_ram)),
        _ => panic!(
            "Unsupported cartridge type, please add support for: 0x{:02X}",
            header.cartridge_type
        ),
    }
}
