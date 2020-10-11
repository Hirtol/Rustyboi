//! This is an integration test suite which runs (if specified) all the test roms provided and saves
//! an image of their framebuffer after ~2 million full emulation cycles (note, different from CPU cycles)
//!
//! If this is a second run then the `old` images will be compared to the `new` images via a
//! `Blake2s` hash. Were there to be any files which differ they will be printed to the output.

use std::fs::{copy, create_dir_all, read, read_dir, read_to_string, remove_dir_all, rename, File};
use std::io;

use std::path::{Path, PathBuf};

use crate::display::TEST_COLOURS;
use rustyboi_core::{DmgColor, EmulatorOptionsBuilder};
use std::ffi::{OsStr, OsString};

use crate::options::AppOptions;
use blake2::{Blake2s, Digest};
use image::ImageBuffer;
use rustyboi_core::emulator::{Emulator, CYCLES_PER_FRAME};
use rustyboi_core::hardware::ppu::FRAMEBUFFER_SIZE;
use std::thread::spawn;
use std::time::Instant;

use anyhow::*;
use std::collections::HashMap;

use gumdrop::Options;
use image::imageops::FilterType;
use std::sync::Arc;

mod display;
mod options;

const TESTING_PATH_OLD: &str = "testing_frames/old/";
const TESTING_PATH_CHANGED: &str = "testing_frames/changed/";
const TESTING_PATH_NEW: &str = "testing_frames/new/";

fn main() -> anyhow::Result<()> {
    let options: AppOptions = AppOptions::parse_args_default_or_exit();
    let current_time = Instant::now();

    // Clean out old files.
    remove_dir_all(TESTING_PATH_OLD);
    remove_dir_all(TESTING_PATH_CHANGED);
    // Move the current images, if they exist, into the `old` directory for comparison purposes.
    if Path::new(TESTING_PATH_NEW).exists() {
        rename(TESTING_PATH_NEW, TESTING_PATH_OLD);
    }
    // Create the new dirs or we'll panic in the image creation step.
    create_dir_all(TESTING_PATH_NEW);
    create_dir_all(TESTING_PATH_CHANGED);

    let old_hashes = calculate_hashes(TESTING_PATH_OLD).unwrap_or_default();

    run_test_roms(options.blargg_path, options.mooneye_path, options.boot_rom);

    let new_hashes = calculate_hashes(TESTING_PATH_NEW).unwrap_or_default();

    for (path, hash) in old_hashes {
        if let Some(_) = new_hashes.get(&path).filter(|t| &**t != &hash) {
            println!("Change in file: {:?}", path);
            copy_changed_file(&path);
        }
    }

    println!("Took: {:?}", current_time.elapsed());

    Ok(())
}

fn run_test_roms(blargg_path: impl AsRef<str>, mooneye_path: impl AsRef<str>, bootrom: impl AsRef<Path>) {
    let boot_file = if bootrom.as_ref().exists() {
        read(bootrom.as_ref()).ok()
    } else {
        None
    };

    if !blargg_path.as_ref().is_empty() {
        run_path(blargg_path.as_ref(), boot_file.clone());
    }

    if !mooneye_path.as_ref().is_empty() {
        run_path(mooneye_path.as_ref(), boot_file);
    }
}

pub fn vec_to_bootrom(vec: &Vec<u8>) -> [u8; 256] {
    let mut result = [0u8; 256];

    for (i, instr) in vec.iter().enumerate() {
        result[i] = *instr;
    }

    result
}

/// An incredibly naive way of doing this, by just spawning as many threads as possible for
/// all test roms and running them for ~2 million iterations, or a custom amount if set via config.
///
/// But it works!
fn run_path(path: impl AsRef<str>, boot_rom_vec: Option<Vec<u8>>) {
    let tests = list_files_with_extensions(path.as_ref(), ".gb").unwrap();
    let custom_list = Arc::new(get_custom_list("custom_test_cycles.txt"));
    let mut threads = Vec::with_capacity(100);

    for path in tests {
        let boot_rom = boot_rom_vec.clone();
        let list_copy = custom_list.clone();
        threads.push(spawn(move || {
            let file_stem = path.file_stem().unwrap().to_owned();
            let mut cycles_to_do = 5_000_000;
            let emu_opts = EmulatorOptionsBuilder::new().boot_rom(boot_rom).build();
            let mut emu = Emulator::new(&read(path).unwrap(), emu_opts);

            if let Some(cycles) = list_copy.get(file_stem.to_str().unwrap_or_default()) {
                cycles_to_do = *cycles;
            }

            for _ in 0..cycles_to_do {
                emu.emulate_cycle();
            }

            let mut remaining_cycles_for_frame = (emu.cycles_performed() % CYCLES_PER_FRAME as u64) as i64;

            while remaining_cycles_for_frame > 0 {
                remaining_cycles_for_frame -= emu.emulate_cycle().0 as i64;
            }

            save_image(emu.frame_buffer(), format!("{}.png", file_stem.to_str().unwrap()));
        }));
    }

    for t in threads {
        t.join();
    }
}

/// Lists all files in the provided `path` (if the former is a directory) with the provided
/// `extension`
fn list_files_with_extensions(path: impl AsRef<Path>, extension: impl AsRef<str>) -> anyhow::Result<Vec<PathBuf>> {
    let mut result = Vec::with_capacity(200);
    if path.as_ref().is_dir() {
        for entry in read_dir(path)? {
            let path = entry?.path();
            if path.is_dir() {
                result.extend(list_files_with_extensions(&path, extension.as_ref())?);
            } else if path.to_str().filter(|t| t.ends_with(extension.as_ref())).is_some() {
                result.push(path);
            }
        }
    } else {
        ()
    }
    Ok(result)
}

/// Copy the provided `file_name` from [TESTING_PATH_NEW](const.TESTING_PATH_NEW.html)
/// and [TESTING_PATH_OLD](const.TESTING_PATH_OLD.html)
/// to [TESTING_PATH_CHANGED](const.TESTING_PATH_CHANGED.html)
fn copy_changed_file(file_name: &OsString) {
    for path in read_dir(TESTING_PATH_NEW).unwrap() {
        let path = path.unwrap().path();
        let path_str = path.file_stem().and_then(OsStr::to_str).unwrap();
        if path_str.contains(file_name.to_str().unwrap()) {
            copy(path.clone(), format!("{}{}_new.png", TESTING_PATH_CHANGED, path_str));
        }
    }
    for path in read_dir(TESTING_PATH_OLD).unwrap() {
        let path = path.unwrap().path();
        let path_str = path.file_stem().and_then(OsStr::to_str).unwrap();
        if path_str.contains(file_name.to_str().unwrap()) {
            copy(path.clone(), format!("{}{}_old.png", TESTING_PATH_CHANGED, path_str));
        }
    }
}

/// Calculates all the hashes for any `.png` files in the provided `directory`
///
/// # Returns
///
/// A `HashMap` with the file stem of a `.png` file as it's key, and the hash as the value
fn calculate_hashes(directory: impl AsRef<Path>) -> anyhow::Result<HashMap<OsString, String>> {
    let files = list_files_with_extensions(directory, ".png")?;
    let mut result = HashMap::with_capacity(100);

    if files.is_empty() {
        return Err(anyhow!("There are no image files to hash"));
    }

    for path in files.iter() {
        let mut file = File::open(path)?;
        let mut hasher = Blake2s::new();
        let _n = io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();

        result.insert(path.file_stem().unwrap().to_os_string(), format!("{:x}", hash));
    }

    Ok(result)
}

/// Renders and saves the provided framebuffer to the `file_name`.
fn save_image(framebuffer: &[DmgColor], file_name: impl AsRef<str>) {
    let mut true_image_buffer = vec![0u8; FRAMEBUFFER_SIZE * 3];

    for (i, colour) in framebuffer.iter().enumerate() {
        let colour = TEST_COLOURS.get_color(colour);
        let offset = i * 3;
        true_image_buffer[offset] = colour.0;
        true_image_buffer[offset + 1] = colour.1;
        true_image_buffer[offset + 2] = colour.2;
    }

    let temp_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
        image::ImageBuffer::from_raw(160, 144, true_image_buffer).unwrap();
    let temp_buffer = image::imageops::resize(&temp_buffer, 320, 288, FilterType::Nearest);
    temp_buffer
        .save(format!("{}{}", TESTING_PATH_NEW, file_name.as_ref()))
        .unwrap();
}

/// Returns the entries from the provided `filename` in the format:
///
/// ```text
/// file_name_no_extension=3000
/// ```
///
/// Where `file_name_no_extension` is the ROM, and `3000` is the amount of emulator cycles.
fn get_custom_list(filename: impl AsRef<str>) -> HashMap<String, u32> {
    let mut result = HashMap::with_capacity(10);

    if Path::new(filename.as_ref()).exists() {
        let file_string = read_to_string(filename.as_ref()).unwrap_or_default();
        for line in file_string.lines() {
            let mut name_and_value = line.split("=");
            let name = name_and_value
                .next()
                .expect("The format of the custom list file is not valid!");
            let cycles = name_and_value
                .next()
                .and_then(|val| val.parse::<u32>().ok())
                .expect("The format of the custom list file is not valid!");
            result.insert(name.trim().to_owned(), cycles);
        }
    }

    result
}
