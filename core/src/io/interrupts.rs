use bitflags::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct Interrupts {
    pub interrupt_enable: InterruptFlags,
    pub interrupt_flag: InterruptFlags,
}

impl Interrupts {

    #[inline(always)]
    pub fn insert_interrupt(&mut self, interrupt: InterruptFlags) {
        self.interrupt_flag.insert(interrupt);
    }

    #[inline(always)]
    pub fn remove_interrupt(&mut self, interrupt: InterruptFlags) {
        self.interrupt_flag.remove(interrupt);
    }

    #[inline(always)]
    pub fn overwrite_if(&mut self, value: u8) {
        // The most significant 3 bits *should* be free to be set by the user, however we wouldn't
        // pass halt_bug otherwise so I'm assuming they're supposed to be unmodifiable
        // by the user after all, and instead are set to 1.
        self.interrupt_flag = InterruptFlags::from_bits_truncate(0xE0 | value);
        //log::info!("Writing interrupt flag {:?} from value: {:02x}", self.interrupts.interrupt_flag, value);
    }

    #[inline(always)]
    pub fn overwrite_ie(&mut self, value: u8) {
        self.interrupt_enable = InterruptFlags::from_bits_truncate(value);
    }

    /// Check if `IF != 0` and that the corresponding bit is also set in `IE`
    #[inline(always)]
    pub fn interrupts_pending(&self) -> bool {
        (self.interrupt_flag.bits & self.interrupt_enable.bits & 0x1F) != 0
    }

    /// Test if the provided `interrupt` should be triggered.
    pub fn interrupt_should_trigger(&self, interrupt: InterruptFlags) -> bool {
        !(interrupt & self.interrupt_flag & self.interrupt_enable).is_empty()
    }

    pub fn get_highest_priority(&self) -> InterruptFlags {
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
            InterruptFlags::NONE
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
        /// Unused
        const UNUSED = 0b1110_0000;
        /// Enum state to represent no interrupt
        const NONE   = 0b0000_0000;
    }
}
