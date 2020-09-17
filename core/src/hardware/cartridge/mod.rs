use crate::hardware::cartridge::header::CartridgeHeader;
use crate::hardware::cartridge::mbc::{MBC0, MBC1};
use bitflags::_core::fmt::{Debug, Formatter};
use std::fmt;

mod header;
mod mbc;

pub trait MBC {
    fn read_3fff(&self, address: u16) -> u8;
    fn read_7fff(&self, address: u16) -> u8;
    fn read_ex_ram(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);
}

pub struct Cartridge {
    header: CartridgeHeader,
    mbc: Box<dyn MBC>,
}

impl Cartridge {
    pub fn new(rom: &[u8]) -> Self {
        let header = CartridgeHeader::new(rom);
        let mbc = create_mbc(&header, rom);
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
        &self.cartridge_header()
    }
}

impl Debug for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cartridge {{ header: {:?} }}", self.header)
    }
}

fn create_mbc(header: &CartridgeHeader, rom: &[u8]) -> Box<dyn MBC> {
    let rom_vec = rom.to_vec();

    log::debug!("Loading ROM with type: 0x{:02X}", header.cartridge_type);

    match header.cartridge_type {
        0x0 => Box::new(MBC0::new(rom_vec)),
        0x1 => Box::new(MBC1::new(rom_vec, false, &header.ram_size)),
        // Potentially need to specify RAM + Battery for MBC1.
        0x2 => Box::new(MBC1::new(rom_vec, false, &header.ram_size)),
        0x3 => Box::new(MBC1::new(rom_vec, true, &header.ram_size)),
        _ => panic!(
            "Unsupported cartridge type, please add support for: 0x{:02X}",
            header.cartridge_type
        ),
    }
}
