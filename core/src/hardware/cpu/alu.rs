use crate::hardware::cpu::traits::{SetU8, ToU8};
use crate::hardware::cpu::CPU;

impl CPU {
    /// Rotate the `target`
    pub(crate) fn rotate_left<T: Copy>(&mut self, target: T)
    where
        Self: ToU8<T>,
        Self: SetU8<T>,
    {
        let value = self.read_u8_value(target);
        let new_value = value.rotate_left(1);
        self.registers.set_zf(new_value == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_cf((value & 0x80) != 0);

        self.set_u8_value(target, new_value);
    }
}
