use super::modules::{ModuleFactory, ModuleFactoryTrait, ModuleTrait};
use futures::future::{BoxFuture, FutureExt};
use std::env::current_dir;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::{Path, PathBuf};

pub struct Fs {
    persistent_data_directory: PathBuf,
    persistent_storage_directory: PathBuf,
    temporary_storage_directory: PathBuf,
}
impl Fs {
    pub fn new() -> Self {
        let persistent_root = current_dir().unwrap().join(Path::new("data")); // FIXME
        create_dir_all(&persistent_root).unwrap();

        let temporary_root = Path::new("/dev/shm/LogicBlocks"); // FIXME
        if temporary_root.exists() {
            remove_dir_all(temporary_root).unwrap();
        }
        create_dir_all(&temporary_root).unwrap();

        let persistent_data_directory = persistent_root.join(Path::new("data"));
        create_dir_all(&persistent_data_directory).unwrap();

        let persistent_storage_directory = persistent_root.join(Path::new("storage"));
        create_dir_all(&persistent_storage_directory).unwrap();

        let temporary_storage_directory = temporary_root.join(Path::new("storage"));
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
impl ModuleTrait for Fs {
}
impl ModuleFactoryTrait for Fs {
    fn spawn<'mf>(_module_factory: &'mf ModuleFactory) -> BoxFuture<'mf, Self> {
        async move { Self::new() }.boxed()
    }
}
