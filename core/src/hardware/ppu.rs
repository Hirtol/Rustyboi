pub const RESOLUTION_WIDTH: usize = 160;
pub const RESOLUTION_HEIGHT: usize = 144;
pub const RGB_CHANNELS: usize = 3;
pub const FRAMEBUFFER_SIZE: usize = RESOLUTION_HEIGHT * RESOLUTION_WIDTH * RGB_CHANNELS;

pub struct PPU {
    pub frame_buffer: [u8; FRAMEBUFFER_SIZE],

}

impl PPU {

}
