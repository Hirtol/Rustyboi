//! This is an integration test suite which runs (if specified) all the test roms provided and saves
//! an image of their framebuffer after ~25 million full emulation cycles (note, different from CPU cycles)
//!
//! If this is a second run then the `old` images will be compared to the `new` images via a
//! `Blake2s` hash. Were there to be any files which differ they will be printed to the output.

use std::fs::{File, read_dir, read, rename, remove_dir, remove_dir_all, create_dir_all};
use std::{io, env};
use std::io::Read;
use std::path::{Path, PathBuf};

use clap::Clap;
use std::ffi::{OsStr, OsString};
use rustyboi_core::hardware::ppu::palette::DmgColor;
use crate::display::{DisplayColour, RGB, TEST_COLOURS};
use rustyboi_core::hardware::cartridge::Cartridge;
use rustyboi_core::emulator::{Emulator, CYCLES_PER_FRAME};
use std::time::Instant;
use std::thread::{Thread, spawn};
use rustyboi_core::hardware::ppu::FRAMEBUFFER_SIZE;
use image::ImageBuffer;
use crate::options::Options;
use blake2::{Blake2s, Digest};
use std::iter::Map;
use std::collections::HashMap;
use anyhow::*;
use blake2::digest::Output;

mod display;
mod options;

const TESTING_PATH_OLD: &str = "testing_frames/old/";
const TESTING_PATH_NEW: &str = "testing_frames/new/";

fn main() -> anyhow::Result<()>{
    let options: Options = Options::parse();
    // Clean out old files.
    remove_dir_all(TESTING_PATH_OLD);
    // Move the current images, if they exist, into the `old` directory for comparison purposes.
    if Path::new(TESTING_PATH_NEW).exists() {
        rename(TESTING_PATH_NEW, TESTING_PATH_OLD);
    }
    // Create the new dir or we'll panic in the image creation step.
    create_dir_all(TESTING_PATH_NEW);

    let old_hashes = calculate_hashes(TESTING_PATH_OLD).unwrap_or_default();

    run_test_roms(options.blargg_path, options.mooneye_path)?;

    let new_hashes = calculate_hashes(TESTING_PATH_NEW).unwrap_or_default();

    for (path, hash) in old_hashes {
        // Can safely unwrap since we know that any old hashes will be in the new hashes
        if let Some(_) = new_hashes.get(&path).filter(|t| &**t != &hash) {
            println!("Change in file: {:?}", path);
        }
    }

    Ok(())
}

fn calculate_hashes(directory: impl AsRef<Path>) -> anyhow::Result<HashMap<OsString, String>> {
    let files = list_files_with_extensions(directory, ".png")?;
    let mut result = HashMap::with_capacity(100);

    if files.is_empty() {
        return Err(anyhow!("There are no image files to hash"));
    }

    for path in files.iter() {
        let mut file = File::open(path)?;
        let mut hasher = Blake2s::new();
        let n = io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();

        result.insert(path.file_stem().unwrap().to_os_string(), format!("{:x}", hash));
    }

    Ok(result)
}

fn run_test_roms(blargg_path: impl AsRef<str>, mooneye_path: impl AsRef<str>) -> anyhow::Result<()>{
    if !blargg_path.as_ref().is_empty() {
        run_path(blargg_path.as_ref());
    }

    if !mooneye_path.as_ref().is_empty() {
        run_path(mooneye_path.as_ref());
    }

    Ok(())
}

/// An incredibly naive way of doing this, by just spawning as many threads as possible for
/// all test roms and running them for ~25 million iterations.
///
/// But it works!
fn run_path(path: impl AsRef<str>) {
    let tests = list_files_with_extensions(path.as_ref(), ".gb").unwrap();
    let mut threads = Vec::with_capacity(100);

    for path in tests {
        threads.push(spawn(move || {
            let file_stem = path.file_stem().unwrap().to_owned();

            let mut emu = Emulator::new(Option::None, &read(path).unwrap());

            for _ in 0..25_000_000 {
                emu.emulate_cycle();
            }

            let mut remaining_cycles_for_frame = (emu.cycles_performed() % CYCLES_PER_FRAME as u128) as i128;

            while remaining_cycles_for_frame > 0 {
                remaining_cycles_for_frame -= emu.emulate_cycle() as i128;
            }

            save_image(&emu.frame_buffer(), format!("{}.png", file_stem.to_str().unwrap()));
        }));
    }

    for t in threads {
        t.join();
    }
}

fn list_files_with_extensions(path: impl AsRef<Path>, extension: impl AsRef<str>) -> anyhow::Result<Vec<PathBuf>> {
    let mut result = Vec::with_capacity(200);
    if path.as_ref().is_dir() {
        for entry in read_dir(path)? {
            let path = entry?.path();
            if path.is_dir() {
                result.extend(list_files_with_extensions(&path, extension.as_ref())?);
            } else if path.to_str().filter(|t| t.ends_with(extension.as_ref())).is_some(){
                result.push(path);
            }
        }
    } else {
        ()
    }
    Ok(result)
}

fn save_image(framebuffer: &[DmgColor], file_name: impl AsRef<str>) {
    let mut true_image_buffer = vec!(0u8; FRAMEBUFFER_SIZE * 3);

    for (i, colour) in framebuffer.iter().enumerate() {
        let colour = TEST_COLOURS.get_color(colour);
        let offset = i * 3;
        true_image_buffer[offset]     = colour.0;
        true_image_buffer[offset + 1] = colour.1;
        true_image_buffer[offset + 2] = colour.2;
    }

    let temp_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>> = image::ImageBuffer::from_raw(160, 144, true_image_buffer).unwrap();

    temp_buffer.save(format!("{}{}", TESTING_PATH_NEW, file_name.as_ref())).unwrap();
}