/// This trait should be used where we might pass either a direct
/// registry address, or a combined registry which points to a memory address.
pub trait ToU8<T: Copy> {
    /// Calling this function should automatically resolve the address directly to
    /// a value, regardless if it was a registry address or a pointer to memory.
    fn get_reg_value(&self, target: T) -> u8;
}

pub trait SetU8<T: Copy, U> {
    fn set_value(&mut self, target: T, value: U);
}