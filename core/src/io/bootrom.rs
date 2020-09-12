

/// 256 bytes total.
pub const BOOTROM_SIZE: usize = 0x100;

type BootRomData = [u8; BOOTROM_SIZE];

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
                data: [0; BOOTROM_SIZE],
            },
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.data[address as usize]
    }
}
