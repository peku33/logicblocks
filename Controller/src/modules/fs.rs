use std::{
    env::current_dir,
    fs::create_dir_all,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Fs {
    persistent_data_directory: PathBuf,
    persistent_storage_directory: PathBuf,
    temporary_storage_directory: PathBuf,
}
impl Fs {
    pub fn new() -> Self {
        // TODO: Make this instance dependant
        let persistent_root = current_dir().unwrap().join("data");
        create_dir_all(&persistent_root).unwrap();

        // TODO: Make this instance dependant
        let temporary_root = if cfg!(unix) {
            if let Ok(temporary_root) = std::env::var("XDG_RUNTIME_DIR") {
                PathBuf::from(temporary_root)
            } else {
                Path::new("/dev/shm").to_path_buf()
            }
        } else {
            current_dir().unwrap().join("temporary")
        };
        let temporary_root = temporary_root.join("logicblocks");
        create_dir_all(&temporary_root).unwrap();

        let persistent_data_directory = persistent_root.join("data");
        create_dir_all(&persistent_data_directory).unwrap();

        let persistent_storage_directory = persistent_root.join("storage");
        create_dir_all(&persistent_storage_directory).unwrap();

        let temporary_storage_directory = temporary_root.join("storage");
        create_dir_all(&temporary_storage_directory).unwrap();

        Self {
            persistent_data_directory,
            persistent_storage_directory,
            temporary_storage_directory,
        }
    }
    pub fn persistent_data_directory(&self) -> &Path {
        &self.persistent_data_directory
    }
    pub fn persistent_storage_directory(&self) -> &Path {
        &self.persistent_storage_directory
    }
    pub fn temporary_storage_directory(&self) -> &Path {
        &self.temporary_storage_directory
    }
}
