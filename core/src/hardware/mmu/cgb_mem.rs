///! In double speed mode the following will operate twice as fast:
/// ```text
///  The CPU (8 MHz, 1 Cycle = approx. 0.5us)
///  Timer and Divider Registers
///  Serial Port (Link Cable)
///  DMA Transfer to OAM
/// ```
use crate::hardware::mmu::INVALID_READ;

#[derive(Default, Debug)]
pub struct CgbData {
    /// Whether double speed mode is currently enabled.
    pub double_speed: bool,
    /// The register to which is written in order to switch speed mode.
    pub prepare_speed_switch: u8,
}

impl CgbData {
    pub fn new() -> Self {
        CgbData { double_speed: false, prepare_speed_switch: 0x7E }
    }

    /// Set the speed and update the `prepare_speed_switch` register.
    /// Will always turn off bit 0 which will make `should_prepare()` return `false`
    pub fn toggle_speed(&mut self) {
        self.double_speed = !self.double_speed;
        self.prepare_speed_switch = if self.double_speed {
            0x80 | (self.prepare_speed_switch & 0x7E)
        } else {
            self.prepare_speed_switch & 0x7E
        }
    }

    pub fn should_prepare(&self) -> bool {
        (self.prepare_speed_switch & 0x01) == 1
    }

    pub fn read_prepare_switch(&self) -> u8 {
        self.prepare_speed_switch
    }

    pub fn write_prepare_switch(&mut self, value: u8) {
        self.prepare_speed_switch = (self.prepare_speed_switch & 0x80) | (value & 0x7F);
    }
}