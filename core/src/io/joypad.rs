use bitflags::_core::fmt::Formatter;
use bitflags::*;

pub const JOYPAD_REGISTER: u16 = 0xFF00;

#[derive(Debug, Copy, Clone)]
pub enum InputKeys {
    START(bool),
    SELECT(bool),
    A(bool),
    B(bool),
    UP(bool),
    DOWN(bool),
    LEFT(bool),
    RIGHT(bool),
}

bitflags! {
    #[derive(Default)]
    pub struct JoypadFlags: u8 {
        /// Right or A
        const RIGHT_A = 0b0000_0001;
        /// Left or B
        const LEFT_B    = 0b0000_0010;
        /// Input Up or Select
        const UP_SELECT  = 0b0000_0100;
        /// Input Down or Start
        const DOWN_START = 0b0000_1000;
        /// Select Direction Keys
        const DIRECTION_KEYS = 0b0001_0000;
        /// Select Button Keys
        const BUTTON_KEYS = 0b0010_0000;
    }
}
