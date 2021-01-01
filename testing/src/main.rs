//! This is an integration test suite which runs (if specified) all the test roms provided and saves
//! an image of their framebuffer after a certain amount of frames (default 600 frames)
//!
//! If this is a second run then the `old` images will be compared to the `new` images via a
//! `Blake2s` hash. Were there to be any files which differ they will be printed to the output.

use std::fs::{copy, create_dir_all, read, read_dir, read_to_string, remove_dir_all, rename, File};
use std::io;

use std::path::{Path, PathBuf};

use crate::display::TEST_COLOURS;
use rustyboi_core::EmulatorOptionsBuilder;
use std::ffi::{OsStr, OsString};

use crate::options::AppOptions;
use blake2::{Blake2s, Digest};
use image::ImageBuffer;
use rustyboi_core::gb_emu::{GameBoyEmulator, GameBoyModel};
use rustyboi_core::hardware::ppu::FRAMEBUFFER_SIZE;
use std::thread::spawn;
use std::time::Instant;

use anyhow::*;
use std::collections::{HashMap, HashSet};

use gumdrop::Options;
use image::imageops::FilterType;
use rustyboi_core::gb_emu::GameBoyModel::{CGB, DMG};
use rustyboi_core::hardware::ppu::palette::RGB;
use std::sync::Arc;

mod display;
mod options;

const TESTING_PATH_OLD: &str = "testing_frames/old/";
const TESTING_PATH_CHANGED: &str = "testing_frames/changed/";
const TESTING_PATH_NEW: &str = "testing_frames/new/";
const DMG_RESULTS_DIRECTORY: &str = "dmg/";
const CGB_RESULTS_DIRECTORY: &str = "cgb/";

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

    run_test_roms(&options.test_path, &options.dmg_boot_rom, DMG);
    run_test_roms(&options.test_path, &options.cgb_boot_rom, CGB);

    let new_hashes = calculate_hashes(TESTING_PATH_NEW).unwrap_or_default();

    for (path, hash) in old_hashes.iter() {
        if let Some(_) = new_hashes.get(path).filter(|t| **t != *hash) {
            println!("Change in file: {:?}", path);
            copy_changed_file(path);
        } else if let None = new_hashes.get(path) {
            println!("File no longer available: {:?}", path);
        }
    }

    // Check for newly running ROMS
    if old_hashes.len() < new_hashes.len() {
        let old_keys: HashSet<_> = old_hashes.keys().collect();
        let new_keys: HashSet<_> = new_hashes.keys().collect();
        for path in new_keys.difference(&old_keys) {
            println!("File now available: {:?}", path);
        }
    }

    println!("Took: {:?}", current_time.elapsed());

    Ok(())
}

fn run_test_roms(test_path: impl AsRef<str>, bootrom: impl AsRef<Path>, emulator_mode: GameBoyModel) {
    let boot_file = if bootrom.as_ref().exists() {
        read(bootrom.as_ref()).ok()
    } else {
        None
    };

    if !test_path.as_ref().is_empty() {
        run_path(test_path.as_ref(), boot_file.clone(), emulator_mode);
    }
}

/// An incredibly naive way of doing this, by just spawning as many threads as possible for
/// all test roms and running them for ~600 frames, or a custom amount if set via config.
///
/// But it works!
fn run_path(path: impl AsRef<str>, boot_rom_vec: Option<Vec<u8>>, emulator_mode: GameBoyModel) {
    let file_extension = if emulator_mode.is_dmg() { ".gb" } else { ".gbc" };
    let tests = list_files_with_extensions(path.as_ref(), file_extension).unwrap();
    let custom_list = Arc::new(get_custom_list("custom_test_cycles.txt"));
    let wait_group = crossbeam::sync::WaitGroup::new();

    for path in tests {
        let boot_rom = boot_rom_vec.clone();
        let list_copy = custom_list.clone();
        let wg = wait_group.clone();

        spawn(move || {
            let file_stem = path.file_stem().unwrap().to_owned();
            let mut frames_to_render = 600;
            let mut options_builder = EmulatorOptionsBuilder::new()
                .with_boot_rom(boot_rom)
                .with_display_colour(TEST_COLOURS);

            options_builder = if emulator_mode.is_dmg() {
                options_builder.with_mode(DMG)
            } else {
                options_builder.with_mode(CGB)
            };

            let emu_opts = options_builder.build();
            let mut emu = GameBoyEmulator::new(&read(path).unwrap(), emu_opts);

            if let Some(frames) = list_copy.get(file_stem.to_str().unwrap_or_default()) {
                frames_to_render = *frames;
            }

            for _ in 0..frames_to_render {
                emu.run_to_vblank();
            }

            let file_path = format!(
                "{}{}_{}.png",
                if emulator_mode.is_dmg() {
                    DMG_RESULTS_DIRECTORY
                } else {
                    CGB_RESULTS_DIRECTORY
                },
                file_stem.to_str().unwrap(),
                if emulator_mode.is_dmg() { "dmg" } else { "cgb" }
            );
            save_image(emu.frame_buffer(), file_path);
            drop(wg);
        });
    }

    wait_group.wait();
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
    find_and_copy_file(TESTING_PATH_NEW, file_name, "new");
    find_and_copy_file(TESTING_PATH_OLD, file_name, "old");
}

/// Finds a file specified in `file_name` and copies it to the provided `path`,
/// appending `appending` to the filename.
fn find_and_copy_file(path: impl AsRef<Path>, file_name: &OsStr, appending: impl AsRef<str>) {
    for path_dir in read_dir(path).unwrap() {
        let path = path_dir.unwrap().path();
        if path.is_dir() {
            find_and_copy_file(path, file_name, appending.as_ref());
        } else {
            let path_str = path.file_stem().and_then(OsStr::to_str).unwrap();
            if path_str.contains(file_name.to_str().unwrap()) {
                copy(path.clone(), format!("{}{}_{}.png", TESTING_PATH_CHANGED, path_str, appending.as_ref()));
            }
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
fn save_image(framebuffer: &[RGB], file_name: impl AsRef<str>) {
    let mut true_image_buffer = vec![0u8; FRAMEBUFFER_SIZE * 3];

    for (i, colour) in framebuffer.iter().enumerate() {
        let offset = i * 3;
        true_image_buffer[offset] = colour.0;
        true_image_buffer[offset + 1] = colour.1;
        true_image_buffer[offset + 2] = colour.2;
    }
    let path = format!("{}{}", TESTING_PATH_NEW, file_name.as_ref());
    create_dir_all(Path::new(&path).parent().unwrap());

    let temp_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
        image::ImageBuffer::from_raw(160, 144, true_image_buffer).unwrap();
    let temp_buffer = image::imageops::resize(&temp_buffer, 320, 288, FilterType::Nearest);
    temp_buffer.save(path).unwrap();
}

/// Returns the entries from the provided `filename` in the format:
///
/// ```text
/// file_name_no_extension=3000
/// ```
///
/// Where `file_name_no_extension` is the ROM, and `3000` is the amount of emulator frames to render.
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
