use bitflags::_core::fmt::Formatter;
use bitflags::*;
use std::convert::{TryFrom, TryInto};
use crate::io::joypad::SelectedMode::{DIRECTION, BUTTON, NONE};

pub const JOYPAD_REGISTER: u16 = 0xFF00;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum InputKey {
    START,
    SELECT,
    A,
    B,
    UP,
    DOWN,
    LEFT,
    RIGHT,
}

#[derive(Debug, Clone)]
pub struct JoyPad {
    pressed_buttons: JoypadFlags,
    pressed_directions: JoypadFlags,
    selected_mode: JoypadFlags,
}

impl JoyPad {
    pub fn new() -> Self {
        JoyPad {
            pressed_buttons: JoypadFlags::empty(),
            pressed_directions: JoypadFlags::empty(),
            selected_mode: JoypadFlags::from_bits_truncate(0xFF),
        }
    }

    pub fn get_register(&self) -> u8 {
        !self.selected_mode.bits
    }

    pub fn set_register(&mut self, mode: u8) {
        self.selected_mode = JoypadFlags::from_bits_truncate(!mode);
        self.update_flags();
    }

    pub fn press_key(&mut self, input: InputKey) {
        use InputKey::*;
        match input {
            DOWN | UP | LEFT | RIGHT => self.pressed_directions.insert(input.get_flag_value()),
            A | B | SELECT | START => self.pressed_buttons.insert(input.get_flag_value()),
        }
        self.update_flags();
    }

    pub fn release_key(&mut self, input: InputKey) {
        use InputKey::*;
        match input {
            DOWN | UP | LEFT | RIGHT => self.pressed_directions.remove(input.get_flag_value()),
            A | B | SELECT | START => self.pressed_buttons.remove(input.get_flag_value()),
        }
        self.update_flags()
    }

    fn update_flags(&mut self) {
        self.selected_mode = JoypadFlags::from_bits_truncate(self.selected_mode.bits() & 0b0011_0000);
        if self.selected_mode.contains(JoypadFlags::BUTTON_KEYS) {
            self.selected_mode.insert(self.pressed_buttons);
        }
        if self.selected_mode.contains(JoypadFlags::DIRECTION_KEYS) {
            self.selected_mode.insert(self.pressed_directions);
        }
    }
}

impl InputKey {
    pub fn get_flag_value(&self) -> JoypadFlags {
        match self {
            InputKey::START  | InputKey::DOWN  => JoypadFlags::DOWN_START,
            InputKey::SELECT | InputKey::UP    => JoypadFlags::UP_SELECT,
            InputKey::B      | InputKey::LEFT  => JoypadFlags::LEFT_B,
            InputKey::A      | InputKey::RIGHT => JoypadFlags::RIGHT_A,
        }
    }
}


bitflags! {
    #[derive(Default)]
    struct JoypadFlags: u8 {
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
        /// Unused, but necessary for bits() to return 0xFF
        const UNUSED_0 = 0b0100_0000;
        /// Unused, but necessary for bits() to return 0xFF
        const UNUSED_1 = 0b1000_0000;
    }
}
