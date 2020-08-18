use crate::hardware::cartridge::header::CartridgeHeader;
use bitflags::_core::fmt::{Debug, Formatter};
use std::fmt;

mod header;
mod mbc;

pub trait MBC {}

pub struct Cartridge {
    header: CartridgeHeader,
    rom: Box<[u8]>,
    mbc: Box<dyn MBC>,
}

impl Cartridge {
    pub fn new(rom: &[u8]) -> Self {
        let header = CartridgeHeader::new(rom);
        let mbc = Box::from(create_mbc(&header));
        Self {
            header,
            rom: Box::from(rom),
            mbc,
        }
    }

    pub fn read_0000_3fff(&self, address: u16) -> u8{
        self.rom[address as usize]
    }

    pub fn read_4000_7fff(&self, address: u16) -> u8{
        self.rom[address as usize]
    }

    pub fn write(&self, address: u16){
        unimplemented!("ROM is read only, to be used for bank switching")
    }
}

impl Debug for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cartridge {{ header: {:?} }}", self.header)
    }
}

fn create_mbc(header: &CartridgeHeader) -> impl MBC {
    test {}
}

#[derive(Debug)]
struct test {}

impl MBC for test {}


