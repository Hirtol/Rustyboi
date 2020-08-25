//! The ALU is concerned with the underlying Math operations for
//! CPU instructions which occur more than once (f.e, several bit shifts occur twice)
use crate::hardware::cpu::traits::{SetU8, ToU8};
use crate::hardware::cpu::CPU;
use crate::hardware::memory::MemoryMapper;

impl<M: MemoryMapper> CPU<M> {
    /// Rotate the register `target` left
    /// C <- [7 <- 0] <- [7]
    ///
    /// Flags: `Z00C`
    pub(crate) fn rotate_left<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let new_value = value.rotate_left(1);

        self.set_rotate_flags(new_value, value & 0x80);

        self.set_u8_value(target, new_value);
    }

    /// Rotate bits in register `target` left through carry.
    /// C <- [7 <- 0] <- C
    ///
    /// Flags: `Z00C`
    pub(crate) fn rotate_left_carry<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let new_value = (value.wrapping_shl(1)) | self.registers.cf() as u8;

        self.set_rotate_flags(new_value, value & 0x80);

        self.set_u8_value(target, new_value);
    }

    ///Shift Left Arithmetic register r8.
    /// C <- [7 <- 0] <- 0
    pub(crate) fn shift_left<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let new_value = value.wrapping_shl(1);

        self.set_rotate_flags(new_value, value & 0x80);

        self.set_u8_value(target, new_value);
    }

    /// Rotate register `target` right.
    /// [0] -> [7 -> 0] -> C
    ///
    /// Flags: `Z00C`
    pub(crate) fn rotate_right<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let new_value = value.rotate_right(1);

        self.set_rotate_flags(new_value, value & 0x01);

        self.set_u8_value(target, new_value);
    }

    /// Rotate register `target` right.
    /// C -> [7 -> 0] -> C
    ///
    /// Flags: `Z00C`
    pub(crate) fn rotate_right_carry<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let new_value = ((self.registers.cf() as u8) << 7) | (value.wrapping_shr(1));

        self.set_rotate_flags(new_value, value & 0x01);

        self.set_u8_value(target, new_value);
    }

    /// Shift Right Arithmetic register r8.
    /// 0 -> [7 -> 0] -> C
    pub(crate) fn shift_right<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let new_value = value.wrapping_shr(1);

        self.set_rotate_flags(new_value, value & 0x01);

        self.set_u8_value(target, new_value);
    }

    #[inline]
    fn set_rotate_flags(&mut self, new_value: u8, cf_check: u8) {
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf(cf_check != 0);
    }
}
