use crate::hardware::mmu::cgb_mem::HdmaMode::HDMA;
use crate::hardware::mmu::{Memory, MemoryMapper};
use crate::hardware::ppu::DMA_TRANSFER;
use crate::scheduler::EventType::{DMARequested, DMATransferComplete};

impl Memory {
    /// Starts the sequence of events for an OAM DMA transfer.
    pub fn dma_transfer(&mut self, value: u8) {
        self.io_registers.write_byte(DMA_TRANSFER, value);
        // In case a previous DMA was running we should cancel it.
        self.scheduler.remove_event_type(DMATransferComplete);
        // 4 Cycles after the request is when the DMA is actually started.
        self.scheduler.push_relative(DMARequested, 4);
    }

    pub fn gather_shadow_oam(&self, start_address: usize) -> Vec<u8> {
        (0..0xA0).map(|i| self.read_byte((start_address + i) as u16)).collect()
    }

    /// Required here since the GDMA can write to arbitrary PPU addresses.
    pub fn gdma_transfer(&mut self) {
        log::info!(
            "Performing GDMA from source: [{:#4X}, {:#4X}] to destination: {:#4X}",
            self.hdma.source_address,
            self.hdma.source_address + self.hdma.transfer_size,
            self.hdma.destination_address
        );
        let values_iter = self.gather_gdma_data();

        for (i, value) in values_iter.into_iter().enumerate() {
            self.write_byte(self.hdma.destination_address + i as u16, value);
        }
    }

    fn gather_gdma_data(&self) -> Vec<u8> {
        (self.hdma.source_address..(self.hdma.source_address + self.hdma.transfer_size))
            .map(|i| self.read_byte(i))
            .collect()
    }

    /// Checks, assuming the current PPU mode is `HBLANK`, whether an `HDMA` transfer should
    /// occur at this point in time. If so, it also executes it.
    pub fn hdma_check_and_transfer(&mut self) {
        if self.hdma.transfer_ongoing && self.hdma.current_mode == HDMA {
            log::info!("Performing HDMA transfer");
            if self.hdma.transfer_ongoing {
                self.do_m_cycle();
                // Pass 36 (single speed)/68 (double speed) cycles where the CPU does nothing.
                for _ in 0..(8 << self.get_speed_shift()) {
                    //TODO: Skip ahead, since CPU is halted during transfer.
                    self.do_m_cycle();
                }
            }
            self.hdma_transfer();
        }
    }

    /// Required here since the HDMA can write to arbitrary PPU addresses.
    fn hdma_transfer(&mut self) {
        // We transfer 16 bytes every H-Blank
        let values_iter: Vec<u8> = (self.hdma.source_address..(self.hdma.source_address + 16))
            .map(|i| self.read_byte(i))
            .collect();

        for (i, value) in values_iter.into_iter().enumerate() {
            self.write_byte(self.hdma.destination_address + i as u16, value);
        }

        self.hdma.advance_hdma();
    }
}
