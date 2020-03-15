use crate::modules::fs::Fs;
use crate::modules::sqlite::SQLite;
use crate::modules::{Context, Handle, Module, ModuleFactory};
use chrono::{Datelike, NaiveDateTime, Timelike};
use failure::Error;
use futures::select;
use futures::stream::StreamExt;
use indoc::indoc;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;

// FIXME: Use Tokio RwLock to protect against clearing
pub struct Organizer {
    fs: Handle<Fs>,
    sqlite: Handle<SQLite>,
}
impl Organizer {
    pub fn new(
        fs: Handle<Fs>,
        sqlite: Handle<SQLite>,
    ) -> Self {
        Self { fs, sqlite }
    }

    pub async fn handle_recording(
        &self,
        channel_id: usize,
        temporary_path: &Path,
        mut temporary_path_metadata: Option<Metadata>,
        time_start_utc: NaiveDateTime,
        time_end_utc: NaiveDateTime,
    ) -> Result<usize, Error> {
        // Build target file path
        let storage_relative_path = Self::build_storage_relative_path(channel_id, time_start_utc);
        let storage_path = self
            .fs
            .persistent_storage_directory()
            .join("devices_soft_ipc_organizer")
            .join(storage_relative_path);

        // Create target directory
        fs::create_dir_all(storage_path.parent().unwrap()).await?;

        // Take file metadata
        let temporary_path_metadata = if temporary_path_metadata.is_some() {
            temporary_path_metadata.take().unwrap()
        } else {
            fs::metadata(&temporary_path).await?
        };

        // Move the file to target directory
        Self::move_file(&temporary_path, &storage_path).await?;

        // Store file information in database
        let recording_id = self
            .sqlite
            .query(move |c| -> Result<usize, Error> {
                c.prepare_cached(indoc!(
                    "
                        INSERT INTO
                            devices_soft_ipc_organizer_recordings (
                                channel_id,
                                timestamp_start,
                                timestamp_end,
                                path,
                                size_bytes
                            )
                        VALUES
                            (?, ?, ?, ?, ?)
                        ;
                    "
                ))?
                .execute(rusqlite::params![
                    channel_id as i64,
                    time_start_utc.timestamp(),
                    time_end_utc.timestamp(),
                    storage_path.to_str().unwrap(),
                    temporary_path_metadata.len() as i64,
                ])?;

                Ok(c.last_insert_rowid() as usize)
            })
            .await?;

        Ok(recording_id)
    }

    async fn prepare(&self) -> Result<(), Error> {
        self
            .sqlite
            .query(|c| -> Result<(), Error> {
                c.execute_batch(
                    indoc!(
                        "
                        CREATE TABLE IF NOT EXISTS devices_soft_ipc_organizer_storage_groups (
                            storage_group_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                            name TEXT NOT NULL,
                            size_bytes_max INTEGER NOT NULL
                        );

                        CREATE TABLE IF NOT EXISTS devices_soft_ipc_organizer_channels (
                            channel_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                            name TEXT NOT NULL,
                            storage_group_id REFERENCES devices_soft_ipc_organizer_storage_groups(storage_group_id) ON DELETE RESTRICT ON UPDATE RESTRICT
                        );

                        CREATE TABLE IF NOT EXISTS devices_soft_ipc_organizer_recordings (
                            recording_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                            channel_id REFERENCES devices_soft_ipc_organizer_channels(channel_id) ON DELETE RESTRICT ON UPDATE RESTRICT,
                            path TEXT NOT NULL,
                            size_bytes INTEGER NOT NULL,
                            timestamp_start INTEGER NOT NULL,
                            timestamp_end INTEGER NOT NULL
                        );
                        CREATE INDEX IF NOT EXISTS devices_soft_ipc_organizer_recordings__timestamp_end ON devices_soft_ipc_organizer_recordings (timestamp_end);
                    "
                    ),
                )?;

                Ok(())
            })
            .await?;

        Ok(())
    }
    async fn cleanup(&self) -> Result<(), Error> {
        // TODO:
        Ok(())
    }
    async fn fix(&self) -> Result<(), Error> {
        // TODO:
        Ok(())
    }

    fn build_storage_relative_path(
        channel_id: usize,
        time_start_utc: NaiveDateTime,
    ) -> PathBuf {
        let mut path_buf = PathBuf::new();
        path_buf.push(format!("{:0>2}", time_start_utc.year()));
        path_buf.push(format!("{:0>2}", time_start_utc.month()));
        path_buf.push(format!("{:0>2}", time_start_utc.day()));
        path_buf.push(format!(
            "{:0>2}:{:0>2}:{:0>2}_{}.mp4",
            time_start_utc.hour(),
            time_start_utc.minute(),
            time_start_utc.second(),
            channel_id,
        ));
        path_buf
    }

    // rename does not work across mount-point boundary
    // this tries to move the file and if it fails, does copy + delete
    async fn move_file(
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> Result<(), Error> {
        if let Ok(()) = fs::rename(&from, &to).await {
            return Ok(());
        }

        fs::copy(&from, &to).await?;
        fs::remove_file(&from).await?;

        Ok(())
    }

    async fn run_once(&self) -> Error {
        if let Err(error) = self.prepare().await {
            return error;
        }

        let mut cleanup_timer = tokio::time::interval(Duration::from_secs(60)).fuse();
        let mut fix_timer = tokio::time::interval(Duration::from_secs(60 * 60)).fuse();

        loop {
            select! {
                _ = cleanup_timer.next() => {
                    if let Err(error) = self.cleanup().await {
                        log::error!("cleanup: {}", error);
                    }
                },
                _ = fix_timer.next() => {
                    if let Err(error) = self.fix().await {
                        log::error!("fix: {}", error);
                    }
                },
            }
        }
    }
    pub async fn run(&self) -> Error {
        let error_delay = Duration::from_secs(5);
        loop {
            let error = self.run_once().await;
            log::error!("run_once error: {}", error);
            tokio::time::delay_for(error_delay).await;
        }
    }
}
impl Module for Organizer {}
impl ModuleFactory for Organizer {
    fn spawn(context: &Context) -> Self {
        Organizer::new(context.get::<Fs>(), context.get::<SQLite>())
    }
}

#[cfg(test)]
mod tests_organizer_inner {
    use super::Organizer;

    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    #[test]
    fn test_build_storage_relative_path_1() {
        let storage_relative_path = Organizer::build_storage_relative_path(
            1234,
            NaiveDateTime::new(
                NaiveDate::from_ymd(2020, 3, 4),
                NaiveTime::from_hms(5, 6, 7),
            ),
        );
        assert_eq!(
            storage_relative_path.to_str().unwrap(),
            "2020/03/04/05:06:07_1234.mp4"
        )
    }
}
