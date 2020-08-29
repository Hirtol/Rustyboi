use super::*;

const VRAM_SIZE: usize = 0x2000;

struct Vram([u8; VRAM_SIZE]);

impl Vram {
    pub fn new(field0: [u8; VRAM_SIZE]) -> Self {
        Vram(field0)
    }
}
