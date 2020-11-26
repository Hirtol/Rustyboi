/// 256 bytes total for DMG
pub const BOOTROM_SIZE_DMG: usize = 0x100;
pub const BOOTROM_SIZE_CGB: usize = 0x900;

type BootRomData = Vec<u8>;

pub struct BootRom {
    pub is_finished: bool,
    data: BootRomData,
}

impl BootRom {
    pub fn new(data: Option<BootRomData>) -> Self {
        match data {
            Some(rom) => Self {
                is_finished: false,
                data: rom,
            },
            None => Self {
                is_finished: true,
                data: Vec::new(),
            },
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.data[address as usize]
    }
}
