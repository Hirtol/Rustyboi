use crate::hardware::cartridge::header::CartridgeHeader;
use bitflags::_core::fmt::{Debug, Formatter};
use std::fmt;

mod header;
mod mbc;

pub trait MBC {
    fn read_byte(&self) -> u8;
    fn write_byte(&mut self);
}

pub struct Cartridge {
    header: CartridgeHeader,
    rom: Box<[u8]>,
    mbc: Box<dyn MBC>,
}

impl Cartridge {
    pub fn new(rom: &[u8]) -> Self {
        let header = CartridgeHeader::new(rom);
        let mbc = Box::from(create_mbc(&header, rom));
        Self {
            header,
            rom: Box::from(rom),
            mbc,
        }
    }

    pub fn read_0000_3fff(&self, address: u16) -> u8 {
        self.rom[address as usize]
    }

    pub fn read_4000_7fff(&self, address: u16) -> u8 {
        self.rom[address as usize]
    }

    pub fn write(&self, address: u16) {
        log::debug!("Writing to ROM address: 0x{:04X}", address);
        //unimplemented!("ROM is read only, to be used for bank switching")
    }
}

impl Debug for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cartridge {{ header: {:?} }}", self.header)
    }
}

fn create_mbc(header: &CartridgeHeader, rom: &[u8]) -> impl MBC {
    test {}
}

#[derive(Debug)]
struct test {}

impl MBC for test {
    fn read_byte(&self) -> u8 {
        unimplemented!()
    }

    fn write_byte(&mut self) {
        unimplemented!()
    }
}
