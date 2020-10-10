use crate::hardware::ppu::PPU;
use crate::scheduler::Scheduler;
use crate::hardware::ppu::tiledata::SpriteAttribute;
use crate::scheduler::EventType::DMATransferComplete;
use crate::hardware::ppu::register_flags::AttributeFlags;

impl PPU {
    /// Called 644 cycles after the start of an OAM DMA transfer.
    pub fn oam_dma_finished(&mut self) {
        self.oam_transfer_ongoing = false;
    }

    /// More efficient batch operation for DMA transfer.
    pub fn oam_dma_transfer(&mut self, values: &[u8], scheduler: &mut Scheduler) {
        //0xFE9F+1-0xFE00 = 0xA0 for OAM size
        if values.len() != 0xA0 {
            panic!("DMA transfer used with an uneven amount of bytes.");
        }

        for i in 0..40 {
            let multiplier = i * 4;
            let current_sprite = &mut self.oam[i];
            current_sprite.y_pos = values[multiplier];
            current_sprite.x_pos = values[multiplier + 1];
            current_sprite.tile_number = values[multiplier + 2];
            current_sprite.attribute_flags = AttributeFlags::from_bits_truncate(values[multiplier + 3]);
        }

        // The OAM transfer takes 644(+4) cycles. (+ 4 cycles delay before you start the dma transfer)
        self.oam_transfer_ongoing = true;
        // In case another DMA transfer was ongoing we first need to cancel that:
        scheduler.push_relative(DMATransferComplete, 644);
    }
}