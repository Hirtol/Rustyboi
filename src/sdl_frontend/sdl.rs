use sdl2::render::{WindowCanvas, Texture};
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum::RGB24;
use rustyboi_core::hardware::ppu::palette::RGB;

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
    pixel_buffer: &[RGB],
) {
    texture.with_lock(Option::None, |arr, _pitch| {
        for (i, colour) in pixel_buffer.iter().enumerate() {
            let offset = i * 3;
            arr[offset] = colour.0;
            arr[offset + 1] = colour.1;
            arr[offset + 2] = colour.2;
        }
    });

    canvas.copy(&texture, None, None);
}