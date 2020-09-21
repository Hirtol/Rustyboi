use crate::hardware::apu::channels::Voice1;

mod channels;


pub struct APU {
    voice1: Voice1,
}