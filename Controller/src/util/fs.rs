use anyhow::{ensure, Context, Error};
use std::path::Path;
use tokio::fs;

// rename does not work across mount-point boundary
// this tries to move the file and if it fails, does copy + delete
pub async fn move_file(
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
) -> Result<(), Error> {
    if let Ok(()) = fs::rename(&from, &to).await.context("rename") {
        return Ok(());
    }

    fs::copy(&from, &to).await.context("copy")?;
    fs::remove_file(&from).await.context("remove_file")?;

    Ok(())
}

pub async fn remove_all_dir_empty(
    root: &Path,
    root_relative_path: &Path,
) -> Result<(), Error> {
    let mut path = root_relative_path;
    loop {
        let path_absolute = root.join(path);

        let metadata = fs::metadata(&path_absolute).await.context("metadata")?;
        ensure!(metadata.is_dir(), "path should be directory");

        let is_empty = fs::read_dir(&path_absolute)
            .await
            .context("read_dir")?
            .next_entry()
            .await
            .context("next_entry")?
            .is_none();
        if !is_empty {
            break;
        }

        fs::remove_dir(&path_absolute).await.context("remove_dir")?;

        path = match path.parent() {
            Some(parent) => parent,
            None => break,
        }
    }

    Ok(())
}
