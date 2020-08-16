/// This trait should be used where we might pass either a direct
/// registry address, or a combined registry which points to a memory address.
///
/// In hindsight, could've probably also just used `Into`
pub trait ToU8<T: Copy> {
    /// Calling this function should automatically resolve the address directly to
    /// a value, regardless if it was a registry address or a pointer to memory.
    fn read_u8_value(&mut self, target: T) -> u8;
}

pub trait SetU8<T: Copy> {
    fn set_u8_value(&mut self, target: T, value: u8);
}

pub trait ToU16<T: Copy> {
    fn get_u16_value(&mut self, target: T) -> u16;
}

pub trait SetU16<T: Copy> {
    fn set_u16_value(&mut self, target: T, value: u16);
}
