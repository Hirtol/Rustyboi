use core::mem;
use rustyboi_core::hardware::ppu::palette::RGB;
use rustyboi_core::hardware::ppu::{FRAMEBUFFER_SIZE, RESOLUTION_WIDTH};
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum::RGB24;
use sdl2::render::{Texture, WindowCanvas};

pub fn setup_sdl(canvas: &mut WindowCanvas) -> Texture {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    // Ensure aspect ratio is kept, in the future we could change this if we want more GUI elements.
    // Or just render ImGui on top ㄟ( ▔, ▔ )ㄏ
    canvas.set_logical_size(160, 144).unwrap();
    canvas.set_scale(1.0, 1.0).unwrap();

    canvas.present();
    canvas.create_texture_streaming(RGB24, 160, 144).unwrap()
}

/// This function assumes pixel_buffer size * 3 == texture buffer size, otherwise panic
pub fn fill_texture_and_copy(canvas: &mut WindowCanvas, texture: &mut Texture, pixel_buffer: &[RGB; FRAMEBUFFER_SIZE]) {
    texture.update(None, transmute_framebuffer(pixel_buffer), RESOLUTION_WIDTH * 3);

    canvas.copy(&texture, None, None);
}

/// Real dirty way of doing this, but the most performant way I've found so far.
/// Instead of copying the buffer twice we just reinterpret the reference to refer to a
/// `u8` RGB array.
pub fn transmute_framebuffer(pixel_buffer: &[RGB]) -> &[u8] {
    unsafe { mem::transmute(pixel_buffer) }
}
