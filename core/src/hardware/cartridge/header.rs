use bitflags::_core::str::from_utf8;
use std::path::Path;
use std::fs::File;

pub const HEADER_START: u16 = 0x0100;
pub const HEADER_END: u16 = 0x014F;

#[derive(Debug, Default)]
pub struct CartridgeHeader {
    /// Upper case ASCII, 16 characters in DMG, zero filled if less than that.
    /// In CGB it's either 15 or 11 characters instead
    pub title: String,
    /// In DMG this is still part of the title bytes, in CGB it contains a flag for determining
    /// The relevant mode.
    pub cgb_flag: bool,
    /// Two character ASCII code, this one is for newer games only. Older games use the other header.
    pub new_licensee_code: u16,
    /// Specifies whether the game supports SGB functions.
    pub sgb_flag: bool,
    /// Specifies which Memory Bank Controller (if any) is used in the cartridge,
    /// and if further external hardware exists in the cartridge.
    pub cartridge_type: u8,
    /// Specifies the ROM Size of the cartridge. Typically calculated as "32KB shl N".
    pub rom_size: u8,
    /// Specifies the size of the external RAM in the cartridge (if any).
    pub ram_size: u8,
    /// Specifies if this version of the game is supposed to be sold in Japan,
    /// or anywhere else. Only two values are defined.
    pub is_japanese: bool,
    /// Specifies the games company/publisher code in range 00-FFh.
    /// A value of 0x33 signalizes that the New License Code in header bytes 0144-0145 is used instead.
    pub old_licensee_code: u8,
    /// Specifies the version number of the game. That is usually 0x00.
    pub mask_rom_version_number: u8,
    /// Contains an 8 bit checksum across the cartridge header bytes 0134-014C.
    /// The checksum is calculated as follows:
    /// `x=0:FOR i=0134h TO 014Ch:x=x-MEM[i]-1:NEXT`
    /// The lower 8 bits of the result must be the same than the value in this entry.
    /// The GAME WON'T WORK if this checksum is incorrect.
    pub header_checksum: u8,
    /// Contains a 16 bit checksum (upper byte first) across the whole cartridge ROM.
    /// Produced by adding all bytes of the cartridge (except for the two checksum bytes).
    /// The Game Boy doesn't verify this checksum.
    pub global_checksum: u16,
}

impl CartridgeHeader {
    pub fn new(rom: &[u8]) -> Self{
        let is_cgb_rom = read_cgb_flag(rom);
        CartridgeHeader{
            title: read_title(rom, is_cgb_rom),
            cgb_flag: is_cgb_rom,
            new_licensee_code: read_new_licensee(rom),
            sgb_flag: read_sgb_flag(rom),
            cartridge_type: read_cartridge_type(rom),
            rom_size: read_rom_size(rom),
            ram_size: read_ram_size(rom),
            is_japanese: read_dest_code(rom),
            old_licensee_code: read_old_licensee(rom),
            mask_rom_version_number: read_mask_rom_version(rom),
            header_checksum: read_header_checksum(rom),
            global_checksum: read_global_checksum(rom),
        }
    }
}

fn read_title(rom: &[u8], cgb_mode: bool) -> String{
    // CGB apparently varies between 11 and 15 characters, chose the pessimistic option here.
    let slice = if cgb_mode { &rom[0x134..=0x13E]} else { &rom[0x134..=0x143]};

    from_utf8(slice)
        .expect("Could not parse title from ROM Header!")
        .trim_matches(char::from(0))
        .to_owned()
}

fn read_cgb_flag(rom: &[u8]) -> bool {
    let cgb_flag = rom[0x143];

    match cgb_flag {
        0x80 => true,
        0xC0 => true,
        _ => false,
    }
}

fn read_new_licensee(rom: &[u8]) -> u16 {
    ((rom[0x144] as u16) << 8) | rom[0x145] as u16
}

fn read_sgb_flag(rom: &[u8]) -> bool {
    let sgb_flag = rom[0x146];

    sgb_flag == 0x03
}

fn read_cartridge_type(rom: &[u8]) -> u8 {
    let c_type = rom[0x147];
    //TODO: Make properly functional.
    c_type
}

fn read_rom_size(rom: &[u8]) -> u8 {
    let r_size = rom[0x148];
    //TODO: Make properly functional.
    r_size
}

fn read_ram_size(rom: &[u8]) -> u8 {
    let r_size = rom[0x149];
    //TODO: Make properly functional.
    r_size
}

fn read_dest_code(rom: &[u8]) -> bool {
    rom[0x14A] == 0x00
}

fn read_old_licensee(rom: &[u8]) -> u8 {
    //TODO: Make functional.
    rom[0x14B]
}

fn read_mask_rom_version(rom: &[u8]) -> u8 {
    rom[0x14C]
}

fn read_header_checksum(rom: &[u8]) -> u8 {
    //TODO: Consider implementing header checksum
    rom[0x14D]
}

fn read_global_checksum(rom: &[u8]) -> u16 {
    ((rom[0x14E] as u16) << 8) | rom[0x14F] as u16
}

#[cfg(test)]
mod tests {
    use crate::hardware::cartridge::header::{CartridgeHeader, read_cgb_flag, read_title};
    use std::io::Read;
    use std::fs::read;

    #[test]
    fn test_read_title(){
        let mut test = vec![0u8; 0x10000];
        for (loc, i) in [0x48,0x65,0x6c,0x6c,0x6f,0x20,0x57,0x6f,0x72,0x00,0x00].iter().enumerate() {
            test[0x134+loc] = *i;
        }
        assert_eq!("Hello Wor", read_title(&test, false))
    }
}