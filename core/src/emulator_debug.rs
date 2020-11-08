use crate::emulator::Emulator;
use crate::hardware::ppu::debugging_features::PaletteDebugInfo;

impl Emulator {
    /// Retrieves and returns all palette info from the `PPU`
    /// Strips out all unnecessary information, only leaving colour info.
    pub fn get_palette_info(&self) -> PaletteDebugInfo {
        PaletteDebugInfo::new(&self.cpu.mmu.ppu)
    }
}