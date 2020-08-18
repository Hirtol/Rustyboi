use crate::hardware::cartridge::header::CartridgeHeader;
use bitflags::_core::fmt::{Debug, Formatter};
use std::fmt;

mod header;

pub trait MBC {

}

pub struct Cartridge{
    header: CartridgeHeader,
    mbc: Box<dyn MBC>,
}

impl Cartridge {
        pub fn new(rom: &[u8]) -> Self {
            let header = CartridgeHeader::new(rom);
            let mbc = Box::from(create_mbc(&header));
            Self {
                header,
                mbc,
        }
    }
}

impl Debug for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cartridge {{ header: {:?} }}", self.header)
    }
}

fn create_mbc(header: &CartridgeHeader) -> impl MBC{
    test{}
}

#[derive(Debug)]
struct test{

}

impl MBC for test {

}


