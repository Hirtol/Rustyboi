# Rustyboi
This is an attempt at yet another GameBoy (just DMG for now) emulator written in Rust.  
This is not supposed to be anything but a learning experience for both creating something in Rust, 
as well as learning about the GB. For actual emulation purposes
one would best refer to another fully working emulator, e.g [PyBoy](https://github.com/Baekalfen/PyBoy).

## Prerequisites
- [SDL2](https://github.com/Rust-SDL2/rust-sdl2)
- Rust 1.45.2

## Installation
### Build from Source
1. `git clone `
2. `cargo build --release`
3. `cd target/release`
4. Execute `rustyboi.exe` or your OS's equivalent.

## Sources
* https://blog.ryanlevick.com/DMG-01/public/book/introduction.html
* https://gbdev.io/pandocs/
* https://izik1.github.io/gbops/
* https://rednex.github.io/rgbds/gbz80.7.html
* http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf
* https://github.com/Gekkio/mooneye-gb