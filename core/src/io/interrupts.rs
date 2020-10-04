use bitflags::*;

#[derive(Default, Debug, Clone)]
pub struct Interrupts {
    pub interrupt_enable: InterruptFlags,
    pub interrupt_flag: InterruptFlags,
}

impl Interrupts {
    pub fn insert_interrupt(&mut self, interrupt: InterruptFlags) {
        self.interrupt_flag.insert(interrupt);
    }

    /// Check if `IF != 0` and that the corresponding bit is also set in `IE`
    pub fn interrupts_pending(&self) -> bool {
        !(self.interrupt_flag & self.interrupt_enable).is_empty()
    }

    pub fn interrupt_should_trigger(&self, interrupt: InterruptFlags) -> bool {
        !(interrupt & self.interrupt_flag & self.interrupt_enable).is_empty()
    }

    pub fn get_immediate_interrupt(&self) -> InterruptFlags {
        if self.interrupt_should_trigger(InterruptFlags::VBLANK) {
            InterruptFlags::VBLANK
        } else if self.interrupt_should_trigger(InterruptFlags::LCD) {
            InterruptFlags::LCD
        } else if self.interrupt_should_trigger(InterruptFlags::TIMER) {
            InterruptFlags::TIMER
        } else if self.interrupt_should_trigger(InterruptFlags::SERIAL) {
            InterruptFlags::SERIAL
        } else if self.interrupt_should_trigger(InterruptFlags::JOYPAD) {
            InterruptFlags::JOYPAD
        } else {
            panic!("No flag available when immediate_interrupt was called!")
        }
    }
}

bitflags! {
    #[derive(Default)]
    pub struct InterruptFlags: u8 {
        /// V-Blank
        const VBLANK = 0b0000_0001;
        /// LCD Stat
        const LCD    = 0b0000_0010;
        /// Timer
        const TIMER  = 0b0000_0100;
        /// Serial
        const SERIAL = 0b0000_1000;
        /// Joypad
        const JOYPAD = 0b0001_0000;
        /// Unused, not yet sure if necesarry.
        const UNUSED = 0b1110_0000;
    }
}
