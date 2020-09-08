use crate::io::interrupts::{Interrupts, InterruptFlags};
use crate::io::timer::InputClock::{C1024, C256};

/// This register is incremented at rate of 16384Hz (~16779Hz on SGB).
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
/// Several flags to indicate incrementing rate of the timer.
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
    timer_overflowed: bool,
}

impl TimerRegisters {
    pub fn tick_timers(&mut self, delta_cycles: u128) -> Option<InterruptFlags> {
        let mut to_return = None;

        self.divider_cycle_counter += delta_cycles;

        // Divider register will increment at 16_384Hz
        // At 100% speed we should see 4194304 cycles per second.
        // 4194304/16384 = 256 so we want to increment every 256 cycles.
        while self.divider_cycle_counter >= 256 {
            self.divider_cycle_counter -= 256;

            self.divider_register = self.divider_register.wrapping_add(1);
        }

        // Whenever an overflow occurs we delay by 4 cycles (1 nop)
        // We call this tick_timers() every instruction, so we're getting close enough by just delaying
        // it by one tick_timers() iteration as we're doing now.
        if self.timer_overflowed && self.timer_counter == 0 {
            self.timer_counter = self.timer_modulo;
            self.timer_overflowed = false;
            to_return = Some(InterruptFlags::TIMER)
        }

        if self.timer_control.timer_enabled {
            self.timer_cycle_counter += delta_cycles;
            // Increment every xx cycles.
            let threshold= self.timer_control.input_select.to_cycle_count();

            while self.timer_cycle_counter > threshold {
                self.timer_cycle_counter -= threshold;
                self.tick_timer();
            }
        }

        to_return
    }

    fn tick_timer(&mut self){
        let (new_value, overflowed) = self.timer_counter.overflowing_add(1);

        self.timer_counter = new_value;
        // If we overflow, we'll set the timer_counter and send the interrupt in the next iteration.
        self.timer_overflowed = overflowed;
    }

    /// Write to the divider register, this will always reset it to 0x00.
    pub fn set_divider(&mut self) {
        // If we've already halfway passed our cycle count then we'll increase our timer
        // due to the falling edge detector in the DMG.
        if self.timer_cycle_counter >= self.timer_control.input_select.to_cycle_count()/2 {
            log::debug!("Div write timer increment");
            self.tick_timer();
        }

        self.divider_register = 0;
        self.divider_cycle_counter = 0;
        self.timer_cycle_counter = 0;
    }

    pub fn set_timer_control(&mut self, value: u8) {
        let old_control = self.timer_control;
        self.timer_control = TimerControl::from(value);

        // When disabling the timer the DMG will increment the timer register if our system clock
        // was already half way through it's cycle due to the falling edge detector.
        if old_control.timer_enabled
            && !self.timer_control.timer_enabled
            && self.timer_cycle_counter >= self.timer_control.input_select.to_cycle_count()/2 {
            log::debug!("Halfway timer increment");
            self.tick_timer();
        }else if self.timer_cycle_counter < old_control.input_select.to_cycle_count()/2
            && self.timer_cycle_counter >= self.timer_control.input_select.to_cycle_count()/2
            && self.timer_control.timer_enabled {
            // if the old selected bit by the multiplexer was 0, the new one is
            // 1, and the new enable bit of TAC is set to 1, it will increase TIMA.
            // Put another way: If our old control had not yet done half of its cycles
            // but our new control will have done so, then we'll increment our timer.
            log::debug!("Lower timer increment");
            self.tick_timer()
        }
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
        TimerControl{input_select: C256, timer_enabled: false}
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

impl InputClock {
    pub fn to_cycle_count(&self) -> u128 {
        match self {
            InputClock::C16 => {
                16
            },
            InputClock::C64 => {
                64
            },
            InputClock::C256 => {
                256
            },
            InputClock::C1024 => {
                1024
            },
        }
    }
}