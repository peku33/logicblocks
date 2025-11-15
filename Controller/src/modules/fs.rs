use anyhow::{Context, Error, bail};
use fs4::fs_std::FileExt;
use std::{
    env,
    fs::{self, File},
    ops::Deref,
    path::{Path, PathBuf},
    sync::LazyLock,
};

#[derive(Debug)]
pub struct Fs {
    persistent_data_directory: DirectoryLocked,
    persistent_storage_directory: DirectoryLocked,
    temporary_storage_directory: DirectoryLocked,
}
impl Fs {
    pub fn new() -> Result<Self, Error> {
        let persistent_data_directory = DirectoryLocked::initialize(persistent_data_directory())
            .context("persistent_data_directory")?;

        let persistent_storage_directory =
            DirectoryLocked::initialize(persistent_storage_directory())
                .context("persistent_storage_directory")?;

        let temporary_storage_directory =
            DirectoryLocked::initialize(temporary_storage_directory())
                .context("temporary_storage_directory")?;

        Ok(Self {
            persistent_data_directory,
            persistent_storage_directory,
            temporary_storage_directory,
        })
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

fn persistent_directory_resolve() -> PathBuf {
    let persistent_directory =
        if let Ok(persistent_directory) = env::var("LOGICBLOCKS_FS_PERSISTENT_DIRECTORY") {
            PathBuf::from(persistent_directory)
        } else {
            env::current_dir().unwrap().join("data")
        };

    persistent_directory
}
pub fn persistent_directory() -> &'static Path {
    static PERSISTENT_DIRECTORY: LazyLock<PathBuf> =
        LazyLock::<PathBuf>::new(persistent_directory_resolve);

    &PERSISTENT_DIRECTORY
}

fn persistent_data_directory_resolve() -> PathBuf {
    let persistent_data_directory = if let Ok(persistent_data_directory) =
        env::var("LOGICBLOCKS_FS_PERSISTENT_DATA_DIRECTORY")
    {
        PathBuf::from(persistent_data_directory)
    } else {
        persistent_directory().join("data")
    };

    persistent_data_directory
}
pub fn persistent_data_directory() -> &'static Path {
    static PERSISTENT_DATA_DIRECTORY: LazyLock<PathBuf> =
        LazyLock::<PathBuf>::new(persistent_data_directory_resolve);

    &PERSISTENT_DATA_DIRECTORY
}

fn persistent_storage_directory_resolve() -> PathBuf {
    let persistent_storage_directory = if let Ok(persistent_storage_directory) =
        env::var("LOGICBLOCKS_FS_PERSISTENT_STORAGE_DIRECTORY")
    {
        PathBuf::from(persistent_storage_directory)
    } else {
        persistent_directory().join("storage")
    };

    persistent_storage_directory
}
pub fn persistent_storage_directory() -> &'static Path {
    static PERSISTENT_STORAGE_DIRECTORY: LazyLock<PathBuf> =
        LazyLock::<PathBuf>::new(persistent_storage_directory_resolve);

    &PERSISTENT_STORAGE_DIRECTORY
}

fn temporary_storage_directory_resolve() -> PathBuf {
    if let Ok(temporary_storage_directory) = env::var("LOGICBLOCKS_FS_TEMPORARY_STORAGE_DIRECTORY")
    {
        return PathBuf::from(temporary_storage_directory);
    };

    env::temp_dir().join("logicblocks").join("storage")
}
pub fn temporary_storage_directory() -> &'static Path {
    static TEMPORARY_STORAGE_DIRECTORY: LazyLock<PathBuf> =
        LazyLock::<PathBuf>::new(temporary_storage_directory_resolve);

    &TEMPORARY_STORAGE_DIRECTORY
}

#[derive(Debug)]
struct DirectoryLocked {
    path: &'static Path,

    lock_file: File,
}
impl DirectoryLocked {
    const LOCK_FILE_NAME: &'static str = "logicblocks.lock";

    pub fn initialize(path: &'static Path) -> Result<Self, Error> {
        // make sure directory exists
        fs::create_dir_all(path).context("create_dir_all")?;

        // create a lock file
        let lock_file_path = path.join(Self::LOCK_FILE_NAME);
        let lock_file = File::create(lock_file_path).context("lock_file open")?;

        // try locking the file
        if !lock_file
            .try_lock_exclusive()
            .context("try_lock_exclusive")?
        {
            bail!("directory {path:?} is locked by other process!");
        }

        Ok(Self { path, lock_file })
    }
}
impl Deref for DirectoryLocked {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path
    }
}
