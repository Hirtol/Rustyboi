use crate::hardware::apu::APU;
use crate::hardware::apu::channels::Voice1;

impl APU {
    pub fn nr10(&self) -> u8 {
        self.voice1.nr10
    }

    pub fn nr11(&self) -> u8 {
        self.voice1.nr11
    }

    pub fn nr12(&self) -> u8 {
        self.voice1.nr12
    }

    pub fn nr13(&self) -> u8 {
        self.voice1.nr13
    }

    pub fn nr14(&self) -> u8 {
        self.voice1.nr14
    }

    pub fn nr50(&self) -> u8 {
        self.nr50
    }

    pub fn nr51(&self) -> u8 {
        self.nr51
    }

    pub fn nr52(&self) -> u8 {
        self.nr52
    }

    pub fn set_nr10(&mut self, value: u8) {
        self.voice1.nr10 = value;
    }

    pub fn set_nr11(&mut self, value: u8) {
        self.voice1.nr11 = value;
    }

    pub fn set_nr12(&mut self, value: u8) {
        self.voice1.nr12 = value;
    }

    pub fn set_nr13(&mut self, value: u8) {
        self.voice1.nr13 = value;
    }

    pub fn set_nr14(&mut self, value: u8) {
        self.voice1.nr14 = value;
    }

    pub fn set_nr50(&mut self, value: u8) {
        self.nr50 = value;
    }

    pub fn set_nr51(&mut self, value: u8) {
        self.nr51 = value;
    }

    pub fn set_nr52(&mut self, value: u8) {
        // Ignore the lower nibble as it's read only.
        self.nr52 = value & 0xF0;
    }


}