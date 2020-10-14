use crate::hardware::mmu::cgb_mem::HdmaMode::{GDMA, HDMA};
///! In double speed mode the following will operate twice as fast:
/// ```text
///  The CPU (8 MHz, 1 Cycle = approx. 0.5us)
///  Timer and Divider Registers
///  Serial Port (Link Cable)
///  DMA Transfer to OAM
/// ```
use crate::hardware::mmu::INVALID_READ;
use crate::scheduler::{Scheduler, EventType};

#[derive(Default, Debug, Copy, Clone)]
pub struct CgbData {
    /// Whether double speed mode is currently enabled.
    pub double_speed: bool,
    /// The register to which is written in order to switch speed mode.
    pub prepare_speed_switch: u8,
}

impl CgbData {
    pub fn new() -> Self {
        CgbData { double_speed: false, prepare_speed_switch: 0x7E }
    }

    /// Set the speed and update the `prepare_speed_switch` register.
    /// Will always turn off bit 0 which will make `should_prepare()` return `false`
    pub fn toggle_speed(&mut self) {
        self.double_speed = !self.double_speed;
        self.prepare_speed_switch = if self.double_speed {
            0xFE
        } else {
            0x7E
        }
    }

    pub fn should_prepare(&self) -> bool {
        (self.prepare_speed_switch & 0x01) == 1
    }

    pub fn read_prepare_switch(&self) -> u8 {
        self.prepare_speed_switch
    }

    pub fn write_prepare_switch(&mut self, value: u8) {
        self.prepare_speed_switch = (self.prepare_speed_switch & 0x80) | 0b0111_1110 | (value & 0x1);
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum HdmaMode {
    GDMA,
    HDMA,
}

#[derive(Debug, Copy, Clone)]
pub struct HdmaRegister {
    pub current_mode: HdmaMode,
    /// Transfer size in bytes
    pub transfer_size: u16,
    pub source_address: u16,
    pub destination_address: u16,
    hdma_length: u8,
    pub transfer_ongoing: bool,
}

impl HdmaRegister {
    pub fn new() -> Self {
        HdmaRegister {
            current_mode: HdmaMode::HDMA,
            transfer_size: 0,
            source_address: 0,
            destination_address: 0,
            hdma_length: INVALID_READ,
            transfer_ongoing: false
        }
    }

    pub fn hdma5(&self) -> u8 {
        if !self.transfer_ongoing {
            INVALID_READ
        } else {
            (self.transfer_size / 16).wrapping_sub(1) as u8
        }
    }

    /// High byte source address
    pub fn write_hdma1(&mut self, value: u8) {
        self.source_address = ((value as u16) << 8) | (self.source_address & 0xFF);
    }

    /// Low byte source address
    pub fn write_hdma2(&mut self, value: u8) {
        self.source_address = (self.source_address & 0xFF00) | ((value & 0xF0) as u16);
    }

    /// High byte destination address
    pub fn write_hdma3(&mut self, value: u8) {
        // Destination is always in VRAM, so we ensure the top nibble is 0x8
        self.destination_address = 0x8000 | (((value & 0x1F) as u16) << 8) | (self.destination_address & 0xFF);
    }

    /// Low byte destination address
    pub fn write_hdma4(&mut self, value: u8) {
        self.destination_address = (self.destination_address & 0xFF00) | ((value & 0xF0) as u16);
    }

    pub fn write_hdma5(&mut self, value: u8, scheduler: &mut Scheduler) {
        log::warn!("Writing to HDMA 5: {:#X}", value);
        self.hdma_length = value;
        self.transfer_size = ((value & 0x7F) as u16 + 1) * 16;

        if self.transfer_ongoing {
            scheduler.remove_event_type(EventType::GDMATransferComplete);
            if value & 0x80 == 0 {
                // If bit 7 is 0 then we stop the current transfer and return
                self.transfer_ongoing = false;
                return;
            }
            // Else we restart the current transfer with a new size.
            // TODO: What happens to src/dest address? Do we need to reload them?
        } else {
            // Only if we don't restart the current transfer do we want to set a new mode.
            self.current_mode = if value & 0x80 == 0 { GDMA } else { HDMA };
        }

        match self.current_mode {
            GDMA => {
                log::info!("Sending request for GDMA transfer at time: {} for blocks: {}", scheduler.current_time, self.transfer_size / 16);
                scheduler.push_relative(EventType::GDMARequested, 4)
            },
            HDMA => {
                //After writing a value to HDMA5 that starts the HDMA copy, the upper bit
                // (that indicates HDMA mode when set to '1') will be cleared
                self.hdma_length &= 0x7F;
            }
        }

        self.transfer_ongoing = true;
    }

    pub fn complete_transfer(&mut self) {
        self.transfer_ongoing = false;
        self.transfer_size = 0;
        self.hdma_length = 0xFF;
    }

    pub fn advance_hdma(&mut self) {
        self.source_address = self.source_address.wrapping_add(16);
        self.destination_address = self.destination_address.wrapping_add(16);
        self.transfer_size = self.transfer_size.wrapping_sub(16);

        if self.transfer_size == 0 {
            self.complete_transfer();
        }
    }
}