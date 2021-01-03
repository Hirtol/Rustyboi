use directories::ProjectDirs;
use nanoserde::{DeJson, SerJson};
use std::fs::{create_dir_all, read_to_string};
use std::path::Path;
use std::{fs, io};

pub trait Storage {
    fn get_value<T: SerJson + DeJson>(&self, file_name: impl AsRef<Path>) -> Option<T>;
    fn save_value<T: SerJson + DeJson>(&self, file_name: impl AsRef<Path>, to_save: &T) -> io::Result<()>;
    fn get_dirs(&self) -> &ProjectDirs;
}

pub struct FileStorage {
    project_dirs: ProjectDirs,
}

impl FileStorage {
    pub fn new() -> Option<FileStorage> {
        let project_dirs = ProjectDirs::from("", "Hirtol", "Rustyboi")?;
        create_dir_all(project_dirs.config_dir());
        create_dir_all(project_dirs.data_dir());
        Some(FileStorage { project_dirs })
    }
}

impl Storage for FileStorage {
    fn get_value<T: SerJson + DeJson>(&self, file_name: impl AsRef<Path>) -> Option<T> {
        let json = read_to_string(self.project_dirs.config_dir().join(file_name)).ok()?;
        T::deserialize_json(json.as_str()).ok()
    }

    fn save_value<T: SerJson + DeJson>(&self, file_name: impl AsRef<Path>, to_save: &T) -> io::Result<()> {
        let json = T::serialize_json(to_save);
        fs::write(self.project_dirs.config_dir().join(file_name), json)
    }

    fn get_dirs(&self) -> &ProjectDirs {
        &self.project_dirs
    }
}
