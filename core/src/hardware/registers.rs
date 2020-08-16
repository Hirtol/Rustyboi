use bitflags::_core::fmt::Formatter;
use bitflags::*;
use core::fmt;
use std::fmt::Display;

bitflags! {
    #[derive(Default)]
    pub struct Flags: u8 {
        /// Zero Flag
        const ZF = 0b1000_0000;
        /// Add/Sub-Flag (BCD)
        const N =  0b0100_0000;
        /// Half Carry Flag (BCD)
        const H =  0b0010_0000;
        /// Carry Flag
        const CF = 0b0001_0000;
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Reg8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Debug, Copy, Clone)]
pub enum Reg16 {
    AF,
    BC,
    DE,
    HL,
    SP,
}

#[derive(Debug, Default, Clone)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: Flags,
    pub h: u8,
    pub l: u8,
    /// Stack pointer
    pub sp: u16,
    /// Program counter
    pub pc: u16,
}

impl Registers {
    pub fn new() -> Self {
        Registers::default()
    }

    #[inline]
    pub fn af(&self) -> u16 {
        (self.a as u16) << 8 | self.f.bits as u16
    }

    #[inline]
    pub fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    #[inline]
    pub fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    #[inline]
    pub fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = Flags::from_bits_truncate(value as u8);
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    #[inline]
    pub fn zf(&self) -> bool {
        self.f.contains(Flags::ZF)
    }

    #[inline]
    pub fn n(&self) -> bool {
        self.f.contains(Flags::N)
    }

    #[inline]
    pub fn h(&self) -> bool {
        self.f.contains(Flags::H)
    }

    #[inline]
    pub fn cf(&self) -> bool {
        self.f.contains(Flags::CF)
    }

    /// Set the Zero Flag.
    #[inline]
    pub fn set_zf(&mut self, value: bool) {
        self.f.set(Flags::ZF, value);
    }

    /// Set the Add/Sub-Flag (BCD).
    #[inline]
    pub fn set_n(&mut self, value: bool) {
        self.f.set(Flags::N, value);
    }

    /// Set the Half Carry Flag (BCD).
    #[inline]
    pub fn set_h(&mut self, value: bool) {
        self.f.set(Flags::H, value);
    }

    /// Set the Carry Flag.
    #[inline]
    pub fn set_cf(&mut self, value: bool) {
        self.f.set(Flags::CF, value);
    }
}

impl Display for Registers {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PC:{:04x} SP:{:04x} \
            A:{:02x} F:{:08b} B:{:02x} C:{:02x} \
            D:{:02x} E:{:02x} H:{:02x} L:{:02x}",
            self.pc, self.sp, self.a, self.f, self.b, self.c, self.d, self.e, self.h, self.l
        )
    }
}

#[cfg(test)]
mod unit_tests {
    use crate::hardware::registers::Registers;

    #[test]
    fn test_16b_register() {
        let mut register = Registers::new();
        register.set_bc(1890);

        assert_eq!(register.bc(), 1890);
        assert_eq!(register.af(), 0);
    }

    #[test]
    fn test_set_af() {
        let mut register = Registers::new();
        // Ensure that normal insertion works properly
        register.set_af(0x0F20);

        assert_eq!(register.f.bits, 0x20);
        assert_eq!(register.a, 0x0F);

        // Ensure that the lower 4 bit nibble is ignored when we transfer back to a Flags register.
        register.set_af(0x0FFA);

        assert_eq!(register.f.bits, 0xF0);
    }
}
