use crate::hardware::cartridge::header::CartridgeHeader;

mod header;

pub struct Cartridge {
    header: CartridgeHeader,
    mbc: dyn MBC,
}

pub trait MBC {

}