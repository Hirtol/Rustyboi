use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::io::timer::InputClock::C256;
use crate::scheduler::{EventType, Scheduler};

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

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
enum InputClock {
    C16 = 0x1,
    C64 = 0x2,
    C256 = 0x3,
    C1024 = 0x0,
}

#[derive(Debug, Copy, Clone)]
pub struct TimerControl {
    timer_enabled: bool,
    input_select: InputClock,
}

#[derive(Debug, Default)]
pub struct TimerRegisters {
    pub timer_counter: u8,
    pub timer_modulo: u8,
    pub timer_control: TimerControl,
    pub just_overflowed: bool,
    timer_overflowed: bool,
    last_div_reset: u64,
}

impl TimerRegisters {
    pub fn divider_register(&self, scheduler: &Scheduler) -> u8 {
        (self.get_time_passed(scheduler) >> 8) as u8
    }

    /// Is called 4 cycles after an overflow actually occurred by the `Scheduler`.
    pub fn timer_overflow(&mut self, scheduler: &mut Scheduler, interrupts: &mut Interrupts) {
        self.timer_counter = self.timer_modulo;
        self.timer_overflowed = false;
        self.just_overflowed = true;
        interrupts.insert_interrupt(InterruptFlags::TIMER);
        // In the 4 cycles after an overflow certain special options are available
        // See `set_timer_counter()`
        scheduler.push_relative(EventType::TimerPostOverflow, 4);
    }

    fn fallen_sys_clock(&self, old_clock: u16, current_sys_clock: u16, select_bit: u16) -> bool {
        (old_clock & select_bit) != 0 && (current_sys_clock & select_bit) == 0
    }

    pub fn scheduled_timer_tick(&mut self, scheduler: &mut Scheduler) {
        self.push_timer_tick_scheduler(scheduler);
        
        if self.timer_control.timer_enabled {
            self.tick_timer(scheduler);
        }
    }

    pub fn tick_timer(&mut self, scheduler: &mut Scheduler) {
        let (new_value, overflowed) = self.timer_counter.overflowing_add(1);

        self.timer_counter = new_value;
        // If we overflow, we'll set the timer_overflowed and send the interrupt after 4 cycles.
        if overflowed {
            self.timer_overflowed = overflowed;
            scheduler.push_relative(EventType::TimerOverflow, 4);
        }
    }

    /// Write to the `TIMA` register (`timer_counter` internally).
    ///
    /// If written to in the 4 clock period before an overflow interrupt, then the interrupt
    /// will be cancelled.
    pub fn set_timer_counter(&mut self, value: u8, scheduler: &mut Scheduler) {
        // If you write to the TIMA register in the 4 clocks that it has overflowed, but
        // not yet reset then you can prevent the interrupt and TMA load from happening.
        // We check for self.timer_counter == 0 to ensure that we've not JUST loaded TMA
        // into TIMA, for if we did then we should ignore this write.
        if self.timer_overflowed && self.timer_counter == 0 {
            self.timer_overflowed = false;
            scheduler.remove_event_type(EventType::TimerOverflow);
        }

        // If you write to TIMA during the cycle that TMA is being loaded to it [B], the write will be ignored
        // and TMA value will be written to TIMA instead.
        if self.just_overflowed {
            self.timer_counter = self.timer_modulo;
        } else {
            self.timer_counter = value;
        }
    }

    /// Write to the `TMA` register (internally `timer_modulo`) and update
    /// `timer_counter` as appropriate
    pub fn set_tma(&mut self, value: u8) {
        // If TMA is written to during the same period as we overflow this new value is used
        // instead of the 'old' value.
        if self.just_overflowed {
            self.timer_counter = value;
        }
        self.timer_modulo = value;
    }

    /// Write to the divider register, this will always reset it to 0x00.
    pub fn set_divider(&mut self, scheduler: &mut Scheduler) {
        let old_sys_clock = self.get_time_passed(scheduler);

        // If we've already halfway passed our cycle count then we'll increase our timer
        // due to the falling edge detector in the DMG.
        if self.fallen_sys_clock(old_sys_clock, 0, self.timer_control.input_select.to_relevant_bit()) {
            self.tick_timer(scheduler);
        }

        self.last_div_reset = scheduler.current_time;

        scheduler.remove_event_type(EventType::TimerTick);
        self.push_timer_tick_scheduler(scheduler);
    }

    pub fn set_timer_control(&mut self, value: u8, scheduler: &mut Scheduler) {
        let delta_passed = self.get_time_passed(scheduler);

        let old_control = self.timer_control;
        self.timer_control = TimerControl::from(value);
        let old_select_bit = old_control.input_select.to_relevant_bit();
        let select_bit = self.timer_control.input_select.to_relevant_bit();

        // When disabling the timer the DMG will increment the timer register if our system clock
        // was already half way through it's cycle due to the falling edge detector.
        if old_control.timer_enabled && !self.timer_control.timer_enabled && (delta_passed & (select_bit)) != 0 {
            self.tick_timer(scheduler);
        }

        // if the old selected bit by the multiplexer was 0, the new one is
        // 1, and the new enable bit of TAC is set to 1, it will increase TIMA.
        // Put another way: If our old control had not yet done half of its cycles
        // but our new control will have done so, then we'll increment our timer.
        if old_control.timer_enabled
            && self.timer_control.timer_enabled
            && (delta_passed & (old_select_bit)) != 0
            && (delta_passed & (select_bit)) == 0
        {
            self.tick_timer(scheduler)
        }

        //TODO: Currently we seem to be out by one cycle with our new implementation compared
        // to our old one, check with games.
        if old_control.input_select != self.timer_control.input_select {
            scheduler.remove_event_type(EventType::TimerTick);
            self.push_timer_tick_scheduler(scheduler);
        }
    }

    pub fn push_timer_tick_scheduler(&self, scheduler: &mut Scheduler) {
        scheduler.push_relative(EventType::TimerTick, self.timer_control.input_select.to_timer_ticks());
    }

    fn get_time_passed(&self, scheduler: &Scheduler) -> u16 {
        // It's fine if the difference is greater than u16:MAX, as that'll essentially
        // act as a wrap-around.
        (scheduler.current_time - self.last_div_reset) as u16
    }
}

impl TimerControl {
    pub fn to_bits(&self) -> u8 {
        let result = if self.timer_enabled { 0x4 } else { 0 };

        result | self.input_select as u8
    }

    pub fn get_clock_interval(&self) -> u64 {
        self.input_select.to_timer_ticks()
    }
}

impl Default for TimerControl {
    fn default() -> Self {
        TimerControl {
            input_select: C256,
            timer_enabled: false,
        }
    }
}

impl From<u8> for TimerControl {
    fn from(val: u8) -> Self {
        TimerControl {
            timer_enabled: val & 0b0000_0100 > 0,
            input_select: InputClock::from(val),
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
            _ => panic!("Invalid value passed to the InputClock parser."),
        }
    }
}

impl InputClock {
    pub fn to_relevant_bit(&self) -> u16 {
        match self {
            InputClock::C16 => 0x0008,
            InputClock::C64 => 0x0020,
            InputClock::C256 => 0x0080,
            InputClock::C1024 => 0x0200,
        }
    }

    pub fn to_timer_ticks(&self) -> u64 {
        match self {
            InputClock::C16 => 16,
            InputClock::C64 => 64,
            InputClock::C256 => 256,
            InputClock::C1024 => 1024,
        }
    }
}
