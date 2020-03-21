use super::recorder::{Recorder, Segment};
use crate::modules::fs::Fs;
use crate::modules::sqlite::SQLite;
use crate::modules::{Context, Handle, Module, ModuleFactory};
use crate::util::borrowed_async::DerefAsyncFuture;
use crate::util::select_all_empty::select_all_empty;
use crate::util::tokio_cancelable::ThreadedInfiniteToError;
use chrono::{Datelike, NaiveDateTime, Timelike};
use failure::{err_msg, format_err, Error};
use futures::channel::mpsc;
use futures::future::{BoxFuture, FutureExt};
use futures::lock::Mutex;
use futures::select;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use indoc::indoc;
use owning_ref::OwningHandle;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use url::Url;

const SEGMENT_TIME: Duration = Duration::from_secs(60);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 10);
const CLEANUP_SIZE_BYTES_TOTAL_MAX_RATIO: f64 = 0.9;
const CLEANUP_CHUNK_SIZE: usize = 32;

#[derive(Eq, PartialEq, Hash, Debug)]
struct RecorderChannelKey {
    channel_id: usize,
    rtsp_url: Url,
}

#[derive(Debug)]
struct RecorderSegment {
    channel_id: usize,
    segment: Segment,
}

type RecorderRunFuture = BoxFuture<'static, Error>;
type RecorderRunObject = OwningHandle<Box<Recorder>, Box<Mutex<RecorderRunFuture>>>;

struct Worker {
    sqlite: Handle<SQLite>,
    fs: Handle<Fs>,

    recorders_reload_receiver: Mutex<mpsc::UnboundedReceiver<()>>,
    recorder_run_object_by_recorder_channel_key:
        Mutex<HashMap<RecorderChannelKey, RecorderRunObject>>,

    recorder_segment_sender: mpsc::UnboundedSender<RecorderSegment>,
    recorder_segment_receiver: Mutex<mpsc::UnboundedReceiver<RecorderSegment>>,
}
impl Worker {
    pub fn new(
        sqlite: Handle<SQLite>,
        fs: Handle<Fs>,
        recorders_reload_receiver: mpsc::UnboundedReceiver<()>,
    ) -> Self {
        let recorders_reload_receiver = Mutex::new(recorders_reload_receiver);

        let recorder_run_object_by_recorder_channel_key = Mutex::new(HashMap::new());

        let (recorder_segment_sender, recorder_segment_receiver) = mpsc::unbounded();
        let recorder_segment_receiver = Mutex::new(recorder_segment_receiver);

        Self {
            sqlite,
            fs,

            recorders_reload_receiver,
            recorder_run_object_by_recorder_channel_key,

            recorder_segment_sender,
            recorder_segment_receiver,
        }
    }

    async fn prepare(&self) -> Result<(), Error> {
        self
            .sqlite
            .query(|connection| -> Result<(), Error> {
                connection.execute_batch(
                    indoc!(
                        "
                        CREATE TABLE IF NOT EXISTS devices_soft_ipc_manager_storage_groups (
                            storage_group_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                            name TEXT NOT NULL,
                            size_bytes_max INTEGER NOT NULL
                        );

                        CREATE TABLE IF NOT EXISTS devices_soft_ipc_manager_channels (
                            channel_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                            name TEXT NOT NULL,
                            rtsp_url TEXT NOT NULL,
                            storage_group_id REFERENCES devices_soft_ipc_manager_storage_groups(storage_group_id) ON DELETE RESTRICT ON UPDATE RESTRICT,
                            enabled INTEGER NOT NULL
                        );

                        CREATE TABLE IF NOT EXISTS devices_soft_ipc_manager_recordings (
                            recording_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                            channel_id REFERENCES devices_soft_ipc_manager_channels(channel_id) ON DELETE RESTRICT ON UPDATE RESTRICT,
                            path TEXT NOT NULL,
                            size_bytes INTEGER NOT NULL,
                            timestamp_start INTEGER NOT NULL,
                            timestamp_end INTEGER NOT NULL
                        );
                        CREATE INDEX IF NOT EXISTS devices_soft_ipc_manager_recordings__timestamp_end ON devices_soft_ipc_manager_recordings (timestamp_end);
                    "
                    ),
                )?;

                Ok(())
            })
            .await?;

        Ok(())
    }

    async fn recorder_run_object_by_recorder_channel_key_reload(&self) -> Result<(), Error> {
        let mut recorder_channel_keys = self
            .sqlite
            .query(|connection| -> Result<Vec<RecorderChannelKey>, Error> {
                let recorder_channel_keys = connection
                    .prepare_cached(indoc!(
                        "
                            SELECT channel_id, rtsp_url
                            FROM devices_soft_ipc_manager_channels
                            WHERE enabled
                        "
                    ))?
                    .query_and_then(rusqlite::NO_PARAMS, |row| {
                        let recorder_channel_key = RecorderChannelKey {
                            channel_id: row.get_raw_checked(0)?.as_i64()? as usize,
                            rtsp_url: row.get_raw_checked(1)?.as_str()?.parse()?,
                        };
                        Ok(recorder_channel_key)
                    })?
                    .collect::<Result<_, Error>>()?;

                Ok(recorder_channel_keys)
            })
            .await?;

        let mut recorder_run_object_by_recorder_channel_key = self
            .recorder_run_object_by_recorder_channel_key
            .try_lock()
            .unwrap();

        // Remove channels not in a list
        recorder_run_object_by_recorder_channel_key
            .retain(|recorder_channel_key, _| recorder_channel_keys.contains(recorder_channel_key));

        // Keep only channels to add
        recorder_channel_keys.retain(|recorder_channel_key| {
            !recorder_run_object_by_recorder_channel_key.contains_key(recorder_channel_key)
        });

        // Add missing channels
        for recorder_channel_key in recorder_channel_keys {
            let temporary_storage_directory = self
                .fs
                .temporary_storage_directory()
                .join("devices_soft_ipc_manager")
                .join(recorder_channel_key.channel_id.to_string());

            let channel_id = recorder_channel_key.channel_id;
            let segment_sender = Box::pin(self.recorder_segment_sender.clone().with(
                async move |segment| {
                    Ok(RecorderSegment {
                        channel_id,
                        segment,
                    })
                },
            ));

            let recorder = Recorder::new(
                recorder_channel_key.rtsp_url.clone(),
                SEGMENT_TIME,
                temporary_storage_directory,
                segment_sender,
            );

            let recorder_run_object = OwningHandle::new_with_fn(Box::new(recorder), unsafe {
                |recorder_ptr| {
                    let recorder_run_future = (*recorder_ptr).run().boxed();
                    Box::new(Mutex::new(recorder_run_future))
                }
            });

            let insert_result = recorder_run_object_by_recorder_channel_key
                .insert(recorder_channel_key, recorder_run_object);
            assert!(insert_result.is_none());
        }

        Ok(())
    }
    async fn recorder_run_object_by_recorder_channel_key_run(&self) -> (Error, usize) {
        let recorder_run_object_by_recorder_channel_key = self
            .recorder_run_object_by_recorder_channel_key
            .try_lock()
            .unwrap();

        let select_all_empty_future =
            select_all_empty(recorder_run_object_by_recorder_channel_key.values().map(
                |recorder_owned_run| DerefAsyncFuture::new(recorder_owned_run.try_lock().unwrap()),
            ));

        select_all_empty_future.await
    }

    async fn recorder_segment_handle(
        &self,
        recorder_segment: RecorderSegment,
    ) -> Result<usize, Error> {
        // Build target file path
        let storage_relative_path = Self::build_storage_relative_path(
            recorder_segment.channel_id,
            recorder_segment.segment.time_start_utc,
        );
        let storage_path = self.get_storage_root_path().join(storage_relative_path);

        // Create target directory
        fs::create_dir_all(storage_path.parent().unwrap()).await?;

        // Move the file to target directory
        Self::move_file(&recorder_segment.segment.path, &storage_path).await?;

        // Store file information in database
        let recording_id = self
            .sqlite
            .query(move |connection| -> Result<usize, Error> {
                connection
                    .prepare_cached(indoc!(
                        "
                            INSERT INTO devices_soft_ipc_manager_recordings (channel_id, timestamp_start, timestamp_end, path, size_bytes)
                            VALUES (?, ?, ?, ?, ?)
                        "
                    ))?
                    .execute(rusqlite::params![
                        recorder_segment.channel_id as i64,
                        recorder_segment.segment.time_start_utc.timestamp(),
                        recorder_segment.segment.time_end_utc.timestamp(),
                        storage_path.to_str().unwrap(),
                        recorder_segment.segment.metadata.len() as i64,
                    ])?;

                Ok(connection.last_insert_rowid() as usize)
            })
            .await?;

        Ok(recording_id)
    }

    async fn recordings_cleanup(&self) -> Result<(), Error> {
        #[derive(Debug)]
        struct StorageGroupStat {
            storage_group_id: usize,
            size_bytes_total: usize,
            size_bytes_max: usize,
        }

        #[derive(Debug)]
        struct DeletionCandidate {
            recording_id: usize,
            path: PathBuf,
            size_bytes: usize,
        }

        let storage_root_path = self.get_storage_root_path();

        let storage_group_stats = self
            .sqlite
            .query(|connection| -> Result<Vec<StorageGroupStat>, Error> {
                let storage_group_stats = connection
                    .prepare_cached(indoc!(
                        "
                            SELECT storage_group_id, size_bytes_total, size_bytes_max
                            FROM devices_soft_ipc_manager_storage_groups
                            JOIN (
                                SELECT storage_group_id, SUM(size_bytes) AS size_bytes_total
                                FROM devices_soft_ipc_manager_recordings
                                JOIN devices_soft_ipc_manager_channels USING(channel_id)
                                GROUP BY storage_group_id
                            ) USING (storage_group_id)
                        "
                    ))?
                    .query_and_then(rusqlite::NO_PARAMS, |row| {
                        let storage_group_stat = StorageGroupStat {
                            storage_group_id: row.get_raw_checked(0)?.as_i64()? as usize,
                            size_bytes_total: row.get_raw_checked(1)?.as_i64()? as usize,
                            size_bytes_max: row.get_raw_checked(2)?.as_i64()? as usize,
                        };
                        Ok(storage_group_stat)
                    })?
                    .collect::<Result<_, Error>>()?;

                Ok(storage_group_stats)
            })
            .await?;

        for storage_group_stat in storage_group_stats {
            // Skip if size is OK
            if storage_group_stat.size_bytes_total <= storage_group_stat.size_bytes_max {
                continue;
            }

            // Calculate desired size
            let size_bytes_total_desired = (CLEANUP_SIZE_BYTES_TOTAL_MAX_RATIO
                * storage_group_stat.size_bytes_max as f64)
                as usize;

            // This will be decreased after each removed file
            let mut size_bytes_total_estimated = storage_group_stat.size_bytes_total;

            // Loop until size is OK
            let storage_group_id = storage_group_stat.storage_group_id;
            while size_bytes_total_estimated >= size_bytes_total_desired {
                // Pick deletion candidates
                let deletion_candidates = self
                    .sqlite
                    .query(move |connection| -> Result<Vec<DeletionCandidate>, Error> {
                        let deletion_candidates = connection
                            .prepare_cached(indoc!(
                                "
                                    SELECT recording_id, path, size_bytes
                                    FROM devices_soft_ipc_manager_recordings
                                    JOIN devices_soft_ipc_manager_channels USING (channel_id)
                                    WHERE storage_group_id = ?
                                    ORDER BY timestamp_end ASC
                                    LIMIT ?
                                "
                            ))?
                            .query_and_then(
                                rusqlite::params![
                                    storage_group_id as i64,
                                    CLEANUP_CHUNK_SIZE as i64,
                                ],
                                |row| {
                                    let deletion_candidate = DeletionCandidate {
                                        recording_id: row.get_raw_checked(0)?.as_i64()? as usize,
                                        path: row.get_raw_checked(1)?.as_str()?.into(),
                                        size_bytes: row.get_raw_checked(2)?.as_i64()? as usize,
                                    };
                                    Ok(deletion_candidate)
                                },
                            )?
                            .collect::<Result<_, Error>>()?;
                        Ok(deletion_candidates)
                    })
                    .await?;

                // This means we want to remove some files but no candidates exist
                if deletion_candidates.is_empty() {
                    log::warn!("Unable to find deletion_candidates");
                    break;
                }

                // Remove picked files
                for deletion_candidate in deletion_candidates {
                    // Remove file
                    if let Err(error) = fs::remove_file(&deletion_candidate.path).await {
                        log::error!(
                            "failed to remove file {:?} during cleanup: {}",
                            deletion_candidate.path,
                            error
                        );
                    }

                    // Remove file from DB
                    let recording_id = deletion_candidate.recording_id;
                    self.sqlite
                        .query(move |connection| -> Result<(), Error> {
                            let rows = connection
                                .prepare_cached(indoc!(
                                    "
                                    DELETE FROM
                                        devices_soft_ipc_manager_recordings
                                    WHERE
                                        recording_id = ?
                                "
                                ))?
                                .execute(rusqlite::params![recording_id as i64])?;

                            match rows {
                                1 => Ok(()),
                                _ => Err(format_err!("Invalid removed rows count: {}", rows)),
                            }
                        })
                        .await?;

                    // Decrease size
                    size_bytes_total_estimated -= deletion_candidate.size_bytes;

                    // Remove empty directories
                    let deletion_candidate_directory = deletion_candidate
                        .path
                        .parent()
                        .ok_or_else(|| err_msg("missing root"))?;
                    if let Err(error) =
                        Self::remove_empty_dir(&storage_root_path, deletion_candidate_directory)
                            .await
                    {
                        log::error!(
                            "failed to remove directory {:?} during cleanup: {}",
                            deletion_candidate_directory,
                            error
                        );
                    }

                    if size_bytes_total_estimated < size_bytes_total_desired {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn run_once(&self) -> Error {
        if let Err(error) = self.prepare().await {
            return error;
        }

        let mut recorders_reload_receiver = self.recorders_reload_receiver.try_lock().unwrap();
        let mut recorder_segment_receiver = self.recorder_segment_receiver.try_lock().unwrap();
        let mut recordings_cleanup_timer = tokio::time::interval(CLEANUP_INTERVAL).fuse();

        // Initial load
        if let Err(error) = self
            .recorder_run_object_by_recorder_channel_key_reload()
            .await
        {
            return error;
        }

        loop {
            select! {
                recorders_reload = recorders_reload_receiver.next() => {
                    if recorders_reload.is_some() {
                        log::trace!("recorders_reload: begin");
                        if let Err(error) = self.recorder_run_object_by_recorder_channel_key_reload().await {
                            return error;
                        }
                        log::trace!("recorders_reload: end");
                    }
                },
                (channel_error, _) = self.recorder_run_object_by_recorder_channel_key_run().fuse() => {
                    return channel_error;
                },
                recorder_segment = recorder_segment_receiver.next() => {
                    if let Some(recorder_segment) = recorder_segment {
                        log::trace!("recorder_segment_handle: begin: {:?}", recorder_segment);
                        if let Err(error) = self.recorder_segment_handle(recorder_segment).await {
                            return error;
                        }
                        log::trace!("recorder_segment_handle: end");
                    }
                },
                recordings_cleanup = recordings_cleanup_timer.next() => {
                    if recordings_cleanup.is_some() {
                        log::trace!("recordings_cleanup: begin");
                        if let Err(error) = self.recordings_cleanup().await {
                            return error;
                        }
                        log::trace!("recordings_cleanup: end");
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

    fn get_storage_root_path(&self) -> PathBuf {
        self.fs
            .persistent_storage_directory()
            .join("devices_soft_ipc_manager")
    }
    fn build_storage_relative_path(
        channel_id: usize,
        time_start_utc: NaiveDateTime,
    ) -> PathBuf {
        let mut path_buf = PathBuf::new();
        path_buf.push(format!("{:0>3}", channel_id));
        path_buf.push(format!("{:0>2}", time_start_utc.year()));
        path_buf.push(format!("{:0>2}", time_start_utc.month()));
        path_buf.push(format!("{:0>2}", time_start_utc.day()));
        path_buf.push(format!(
            "{:0>2}_{:0>2}_{:0>2}.mp4",
            time_start_utc.hour(),
            time_start_utc.minute(),
            time_start_utc.second(),
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

    // removed all directories from remove_dir up to root_dir if they are empty
    // root_dir must be a parent of remove_dir
    async fn remove_empty_dir(
        root_dir: &Path,
        remove_dir: &Path,
    ) -> Result<(), Error> {
        let mut remove_dir: PathBuf = remove_dir.strip_prefix(root_dir)?.into();
        loop {
            if fs::remove_dir(root_dir.join(&remove_dir)).await.is_err() {
                break;
            }
            if remove_dir.pop() {
                break;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests_worker {
    use super::Worker;

    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    #[test]
    fn test_build_storage_relative_path_1() {
        let storage_relative_path = Worker::build_storage_relative_path(
            1234,
            NaiveDateTime::new(
                NaiveDate::from_ymd(2020, 3, 4),
                NaiveTime::from_hms(5, 6, 7),
            ),
        );
        assert_eq!(
            storage_relative_path.to_str().unwrap(),
            "1234/2020/03/04/05_06_07.mp4"
        )
    }
}

pub struct Manager {
    recorders_reload_sender: mpsc::UnboundedSender<()>,
    run_handle: ThreadedInfiniteToError,
}
impl Manager {
    pub fn new(
        sqlite: Handle<SQLite>,
        fs: Handle<Fs>,
    ) -> Self {
        let (recorders_reload_sender, recorders_reload_receiver) = mpsc::unbounded();
        let run_handle =
            ThreadedInfiniteToError::new("Manager (ipc.soft.devices)".to_owned(), async move {
                let worker = Worker::new(sqlite, fs, recorders_reload_receiver);
                worker.run().await
            });
        Self {
            recorders_reload_sender,
            run_handle,
        }
    }
}
impl Module for Manager {}
impl ModuleFactory for Manager {
    fn spawn(context: &Context) -> Self {
        Self::new(context.get(), context.get())
    }
}
