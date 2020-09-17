pub mod emulator;
pub use crate::hardware::ppu::palette::DmgColor;
pub use crate::io::joypad::InputKey;

pub mod hardware;
mod io;

fn print_array_raw<T: Sized>(array: T) {
    let view = &array as *const _ as *const u8;
    for i in 0..(4 * 40) {
        if i % 16 == 0 {
            println!();
        }
        print!("{:02X} ", unsafe { *view.offset(i) });
    }
    println!();
}
