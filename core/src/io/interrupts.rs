use bitflags::*;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum Interrupts {
    VBLANK = 0b0000_0001,
    LcdStat = 0b0000_0010,
    TIMER = 0b0000_0100,
    SERIAL = 0b0000_1000,
    JOYPAD = 0b0001_0000,
}

impl Interrupts {
    pub fn iter() -> impl Iterator<Item = Interrupts> {
        use crate::io::interrupts::Interrupts::{LcdStat, JOYPAD, SERIAL, TIMER, VBLANK};
        [VBLANK, LcdStat, TIMER, SERIAL, JOYPAD].iter().copied()
    }
}

#[derive(Default, Debug, Clone)]
pub struct InterruptModule {
    pub interrupt_enable: InterruptFlags,
    pub interrupt_flag: InterruptFlags,
    pub pending_interrupts: Vec<Interrupts>,
}

impl InterruptModule {
    pub fn insert_interrupt(&mut self, interrupt: InterruptFlags) {
        self.interrupt_flag.insert(interrupt);
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

impl InterruptFlags {
    pub fn contains_interrupt(&self, interrupt: Interrupts) -> bool {
        self.contains(InterruptFlags::from_bits_truncate(interrupt as u8))
    }

    pub fn iter() -> impl Iterator<Item = InterruptFlags> {
        [
            InterruptFlags::VBLANK,
            InterruptFlags::LCD,
            InterruptFlags::TIMER,
            InterruptFlags::SERIAL,
            InterruptFlags::JOYPAD,
        ]
        .iter()
        .copied()
    }
}

#[cfg(test)]
mod test {
    use super::Interrupts;
    use super::Interrupts::*;

    #[test]
    fn test_interrupt_order() {
        let ordered_array = [VBLANK, LcdStat, TIMER, SERIAL, JOYPAD];
        for (i, interrupt) in Interrupts::iter().enumerate() {
            assert_eq!(ordered_array[i], interrupt)
        }
    }
}
