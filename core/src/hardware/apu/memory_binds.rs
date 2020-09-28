use crate::hardware::apu::channels::Voice1;
use crate::hardware::apu::APU;

impl APU {

    pub fn nr14(&self) -> u8 {
        self.voice1.nr14
    }

    pub fn set_nr14(&mut self, value: u8) {
        self.voice1.nr14 = value;
    }
}
