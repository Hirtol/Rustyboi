use crate::io::interrupts::Interrupts;
use crate::io::timer::InputClock::C1024;

///This register is incremented at rate of 16384Hz (~16779Hz on SGB).
/// Writing any value to this register resets it to 00h.
///
/// Note: The divider is affected by CGB double speed mode, and will increment at 32768Hz in double speed.
pub const DIVIDER_REGISTER: u16 = 0xFF04;
/// This timer is incremented by a clock frequency specified by the TAC register ($FF07).
/// When the value overflows (gets bigger than FFh) then it will be reset to the value
/// specified in TMA (FF06), and an interrupt will be requested, as described below.
pub const TIMER_COUNTER: u16 = 0xFF05;
/// When the TIMA overflows, this data will be loaded.
pub const TIMER_MODULO: u16 = 0xFF06;

pub const TIMER_CONTROL: u16 = 0xFF07;

#[derive(Debug, Copy, Clone)]
enum InputClock {
    C16   = 0x1,
    C64   = 0x2,
    C256  = 0x3,
    C1024 = 0x0,
}

#[derive(Debug, Copy, Clone)]
pub struct TimerControl {
    timer_enabled: bool,
    input_select: InputClock,
}

#[derive(Debug, Default)]
pub struct TimerRegisters {
    pub divider_register: u8,
    pub timer_counter: u8,
    pub timer_modulo: u8,
    pub timer_control: TimerControl,
    divider_cycle_counter: u128,
    timer_cycle_counter: u128,
}

impl TimerRegisters {
    pub fn tick_timers(&mut self, delta_cycles: u128) -> Option<Interrupts> {
        self.divider_cycle_counter += delta_cycles;
        self.timer_cycle_counter += delta_cycles;

        // Divider register will increment at 16_384Hz
        // At 100% speed we should see 4194304 cycles per second.
        // 4194304/16384 = 256 so we want to increment every 256 cycles.
        if self.divider_cycle_counter >= 256 {
            self.divider_cycle_counter -= 256;

            self.divider_register = self.divider_register.wrapping_add(1);
        }

        if self.timer_control.timer_enabled {
            let threshold;
            match self.timer_control.input_select {
                InputClock::C16 => {
                    // Increment every 16 cycles
                    threshold = 16;
                },
                InputClock::C64 => {
                    // Increment every 64 cycles
                    threshold = 64;
                },
                InputClock::C256 => {
                    // Increment every 256 cycles
                    threshold = 256;
                },
                InputClock::C1024 => {
                    // Increment every 1024 cycles
                    threshold = 1024;
                },
            }

            if self.timer_cycle_counter > threshold {
                self.timer_cycle_counter -= threshold;
                return self.tick_timer();
            }
        }

        None
    }

    fn tick_timer(&mut self) -> Option<Interrupts>{
        let (new_value, overflowed) = self.timer_counter.overflowing_add(1);

        if overflowed {
            self.timer_counter = self.timer_modulo;
            Some(Interrupts::TIMER)
        } else {
            self.timer_counter = new_value;
            None
        }
    }

    /// Write to the divider register, this will always reset it to 0x00.
    pub fn set_divider(&mut self) {
        self.divider_register = 0;
    }

    pub fn set_timer_control(&mut self, value: u8) {
        self.timer_control = TimerControl::from(value);
    }
}

impl TimerControl {
    pub fn to_bits(&self) -> u8 {
        let mut result = if self.timer_enabled {0x4} else {0};

        result | self.input_select as u8
    }
}

impl Default for TimerControl {
    fn default() -> Self {
        TimerControl{input_select: C1024, timer_enabled: false}
    }
}

impl From<u8> for TimerControl {
    fn from(val: u8) -> Self {
        TimerControl {
            timer_enabled: val & 0b0000_0100 > 0,
            input_select: InputClock::from(val)
        }
    }
}

impl From<u8> for InputClock {
    fn from(val: u8) -> Self {
        match val & 0x3 {
            0x0 => InputClock::C1024,
            0x1 => InputClock::C16,
            0x2 => InputClock::C64,
            0x3 => InputClock::C256,
            _ => panic!("Invalid value passed to the InputClock parser.")
        }
    }
}