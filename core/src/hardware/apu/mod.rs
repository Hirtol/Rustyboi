use crate::hardware::apu::channels::Voice1;

mod channels;
mod memory_binds;


pub struct APU {
    voice1: Voice1,
    /// The volume bits specify the "Master Volume" for Left/Right sound output.
    /// SO2 goes to the left headphone, and SO1 goes to the right.
    ///
    /// The Vin signal is unused by licensed games (could've been used for 5th voice)
    ///
    ///  ```Bit 7   - Output Vin to SO2 terminal (1=Enable)
    ///  Bit 6-4 - SO2 output level (volume)  (0-7)
    ///  Bit 3   - Output Vin to SO1 terminal (1=Enable)
    ///  Bit 2-0 - SO1 output level (volume)  (0-7)```
    nr50: u8,
    /// Each channel can be panned hard left, center, or hard right.
    ///
    ///  ```Bit 7 - Output sound 4 to SO2 terminal
    ///  Bit 6 - Output sound 3 to SO2 terminal
    ///  Bit 5 - Output sound 2 to SO2 terminal
    ///  Bit 4 - Output sound 1 to SO2 terminal
    ///  Bit 3 - Output sound 4 to SO1 terminal
    ///  Bit 2 - Output sound 3 to SO1 terminal
    ///  Bit 1 - Output sound 2 to SO1 terminal
    ///  Bit 0 - Output sound 1 to SO1 terminal```
    nr51: u8,
    /// Disabling the sound controller by clearing Bit 7 destroys the contents of all sound registers.
    /// Also, it is not possible to access any sound registers (execpt FF26) while the sound controller is disabled.
    /// Bits 0-3 of this register are read only status bits, writing to these bits does NOT enable/disable sound.
    /// The flags get set when sound output is restarted by setting the Initial flag (Bit 7 in NR14-NR44),
    /// the flag remains set until the sound length has expired (if enabled).
    /// A volume envelopes which has decreased to zero volume will NOT cause the sound flag to go off.
    ///
    ///  ```Bit 7 - All sound on/off  (0: stop all sound circuits) (Read/Write)
    ///  Bit 3 - Sound 4 ON flag (Read Only)
    ///  Bit 2 - Sound 3 ON flag (Read Only)
    ///  Bit 1 - Sound 2 ON flag (Read Only)
    ///  Bit 0 - Sound 1 ON flag (Read Only)```
    nr52: u8,
}

impl APU {
    pub fn tick(&mut self, delta_cycles: u64) {
        
    }
}