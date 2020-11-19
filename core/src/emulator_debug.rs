use crate::emulator::{Emulator, GameBoyModel};
use crate::hardware::ppu::debugging_features::PaletteDebugInfo;
use crate::hardware::ppu::tiledata::SpriteAttribute;
use crate::hardware::ppu::palette::RGB;

impl Emulator {
    /// Retrieves and returns all palette info from the `PPU`
    /// Strips out all unnecessary information, only leaving colour info.
    pub fn get_palette_info(&self) -> PaletteDebugInfo {
        PaletteDebugInfo::new(&self.cpu.mmu.ppu, self.emulator_mode())
    }

    pub fn vram_tiles(&self) -> [RGB; 8 * 8 * 768] {
        self.cpu.mmu.ppu.tiles_cgb()
    }

    pub fn oam(&self) -> &[SpriteAttribute; 40] {
        &self.cpu.mmu.ppu.oam
    }

    pub fn emulator_mode(&self) -> GameBoyModel {
        self.cpu.mmu.emulated_model
    }
}