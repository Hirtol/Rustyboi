use sdl2::render::{WindowCanvas, Texture};
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum::RGB24;
use rustyboi_core::hardware::ppu::palette::RGB;
use rustyboi_core::hardware::ppu::{FRAMEBUFFER_SIZE, RESOLUTION_WIDTH};
use core::mem;

pub fn setup_sdl(canvas: &mut WindowCanvas) -> Texture {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    // Ensure aspect ratio is kept, in the future we could change this if we want more GUI elements.
    // Or just render ImGui on top ㄟ( ▔, ▔ )ㄏ
    canvas.set_logical_size(160, 144);
    canvas.set_scale(1.0, 1.0);

    canvas.present();
    canvas.create_texture_streaming(RGB24, 160, 144).unwrap()
}

/// This function assumes pixel_buffer size * 3 == texture buffer size, otherwise panic
pub fn fill_texture_and_copy(
    canvas: &mut WindowCanvas,
    texture: &mut Texture,
    pixel_buffer: &[RGB; FRAMEBUFFER_SIZE],
) {
    texture.update(None, transmute_framebuffer(pixel_buffer), RESOLUTION_WIDTH*3);

    canvas.copy(&texture, None, None);
}

/// Real dirty way of doing this, but the most performant way I've found so far.
/// Instead of copying the buffer twice we just reinterpret the reference to refer to a
/// `u8` RGB array.
fn transmute_framebuffer(pixel_buffer: &[RGB; FRAMEBUFFER_SIZE]) -> &[u8; FRAMEBUFFER_SIZE*3]{
    unsafe {
        mem::transmute(pixel_buffer)
    }
}