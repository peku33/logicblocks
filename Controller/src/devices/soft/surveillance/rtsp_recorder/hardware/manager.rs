use super::channel::ChannelSegment;
use crate::{
    datatypes::ratio::Ratio,
    modules,
    util::{
        async_barrier::Barrier,
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        fs::{move_file, remove_all_dir_empty},
        runnable::{Exited, Runnable},
    },
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use atomic_refcell::AtomicRefCell;
use chrono::{Datelike, Timelike};
use futures::{
    channel::mpsc,
    future::FutureExt,
    join, select,
    stream::{StreamExt, TryStreamExt},
};
use indoc::indoc;
use modules::{fs::Fs, sqlite::SQLite};
use std::{collections::HashMap, fmt, path::PathBuf, rc::Rc, time::Duration};
use tokio::fs;

pub type ChannelId = usize;

#[derive(Debug)]
pub struct ChannelData {
    pub name: String,
    pub detection_threshold: Ratio,
}

#[derive(Debug)]
pub struct ChannelIdSegment {
    pub id: ChannelId,
    pub segment: ChannelSegment,
}

#[derive(Debug)]
pub struct Manager<'f> {
    name: String,
    fs: &'f Fs,

    sqlite: SQLite<'f>,

    initialized: Barrier,

    channel_segment_sender: mpsc::UnboundedSender<ChannelIdSegment>,
    channel_segment_receiver: AtomicRefCell<mpsc::UnboundedReceiver<ChannelIdSegment>>,
}
impl<'f> Manager<'f> {
    pub fn new(
        name: String,
        fs: &'f Fs,
    ) -> Self {
        let sqlite = SQLite::new(format!("rtsp_recorder.manager.{}", name), fs);

        let initialized = Barrier::new();

        let (channel_segment_sender, channel_segment_receiver) =
            mpsc::unbounded::<ChannelIdSegment>();
        let channel_segment_receiver = AtomicRefCell::new(channel_segment_receiver);

        Self {
            name,
            fs,

            sqlite,

            initialized,

            channel_segment_sender,
            channel_segment_receiver,
        }
    }

    // initialization
    async fn initialize_once(&self) -> Result<(), Error> {
        self.sqlite
            .query(|connection| -> Result<(), Error> {
                connection.execute_batch(indoc!("
                    CREATE TABLE IF NOT EXISTS storage_groups (
                        storage_group_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                        name TEXT NOT NULL,
                        size_bytes_max INTEGER NOT NULL,
                        detection_level_to_second_ratio REAL NOT NULL
                    ) STRICT;

                    CREATE TABLE IF NOT EXISTS channels (
                        channel_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                        name TEXT NOT NULL,
                        storage_group_id REFERENCES storage_groups(storage_group_id) ON DELETE RESTRICT ON UPDATE RESTRICT,
                        enabled INTEGER NOT NULL,
                        detection_threshold REAL NOT NULL
                    ) STRICT;

                    CREATE TABLE IF NOT EXISTS recordings (
                        recording_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                        channel_id REFERENCES channels(channel_id) ON DELETE RESTRICT ON UPDATE RESTRICT,
                        timestamp_start INTEGER NOT NULL,
                        timestamp_end INTEGER NOT NULL,
                        detection_level REAL NOT NULL,
                        path_storage_relative TEXT NOT NULL,
                        size_bytes INTEGER NOT NULL
                    ) STRICT;
                "))?;
                Ok(())
            })
            .await
            .context("query")?;

        self.initialized.release();

        Ok(())
    }
    async fn initialize(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Result<(), Exited> {
        const ERROR_DELAY: Duration = Duration::from_secs(5);

        loop {
            let error = match self.initialize_once().await.context("initialize_once") {
                Ok(()) => break Ok(()),
                Err(error) => error,
            };
            log::error!("{}: {:?}", self, error);

            select! {
                () = tokio::time::sleep(ERROR_DELAY).fuse() => {},
                () = exit_flag => break Err(Exited),
            }
        }
    }

    // channels management
    pub async fn channels_data_get(&self) -> Result<HashMap<ChannelId, ChannelData>, Error> {
        self.initialized.waiter().await;

        let channels = self
            .sqlite
            .query(
                #[allow(clippy::type_complexity)]
                |connection| -> Result<Box<[(usize, String, Ratio)]>, Error> {
                    let channels = connection
                        .prepare(indoc!(
                            "
                            SELECT
                                channel_id,
                                name,
                                detection_threshold
                            FROM
                                channels
                            WHERE
                                enabled
                        "
                        ))?
                        .query_map([], |row| {
                            let channel_id = row.get_ref_unwrap(0).as_i64()? as usize;
                            let name = row.get_ref_unwrap(1).as_str()?.to_owned();
                            let detection_threshold =
                                Ratio::from_f64(row.get_ref_unwrap(2).as_f64()?).unwrap();
                            Ok((channel_id, name, detection_threshold))
                        })?
                        .collect::<rusqlite::Result<_>>()?;
                    Ok(channels)
                },
            )
            .await
            .context("query")?
            .into_iter()
            .map(|(channel_id, name, detection_threshold)| {
                (
                    channel_id,
                    ChannelData {
                        name,
                        detection_threshold,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(channels)
    }

    // segment handling
    pub fn channel_segment_sender(&self) -> mpsc::UnboundedSender<ChannelIdSegment> {
        self.channel_segment_sender.clone()
    }
    async fn channel_segment_handle(
        &self,
        channel_id: ChannelId,
        channel_segment: ChannelSegment,
    ) -> Result<(), Error> {
        let segment_path_storage_relative =
            Self::segment_storage_relative_path_build(channel_id, &channel_segment);
        let segment_path_storage = self
            .storage_directory_root_path_build()
            .join(&segment_path_storage_relative);

        fs::create_dir_all(segment_path_storage.parent().unwrap())
            .await
            .context("create_dir_all")?;

        move_file(&channel_segment.segment.path, &segment_path_storage)
            .await
            .context("move_file")?;

        let _recording_id = self
            .sqlite
            .query(move |connection| -> Result<usize, Error> {
                let recording_id = connection
                    .prepare(indoc!("
                        INSERT INTO
                            recordings
                            (channel_id, timestamp_start, timestamp_end, detection_level, path_storage_relative, size_bytes)
                        VALUES
                            (?, ?, ?, ?, ?, ?)
                    "))?
                    .execute(rusqlite::params![
                        channel_id as i64,
                        channel_segment.segment.time_start.timestamp(),
                        channel_segment.segment.time_end.timestamp(),
                        channel_segment.detection_level.to_f64(),
                        segment_path_storage_relative.to_str().unwrap(),
                        channel_segment.segment.metadata.len() as i64,
                    ])?;

                Ok(recording_id)
            })
            .await.context("query")?;

        Ok(())
    }
    async fn channel_segment_receiver_run_once(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        self.channel_segment_receiver
            .borrow_mut()
            .by_ref()
            .stream_take_until_exhausted(exit_flag)
            .map(Result::<_, Error>::Ok)
            .try_for_each_concurrent(None, async |channel_id_segment| {
                self.channel_segment_handle(channel_id_segment.id, channel_id_segment.segment)
                    .await
                    .context("channel_segment_handle")?;

                Ok(())
            })
            .await
            .context("channel_segment_receiver")?;

        Ok(Exited)
    }
    async fn channel_segment_receiver_run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        const ERROR_DELAY: Duration = Duration::from_secs(5);

        loop {
            let error = match self
                .channel_segment_receiver_run_once(exit_flag.clone())
                .await
                .context("channel_segment_receiver_run_once")
            {
                Ok(Exited) => break,
                Err(error) => error,
            };
            log::error!("{}: {:?}", self, error);

            select! {
                () = tokio::time::sleep(ERROR_DELAY).fuse() => {},
                () = exit_flag => break,
            }
        }

        Exited
    }

    // cleanup
    async fn cleanup(&self) -> Result<(), Error> {
        // find recordings to remove
        let recordings_to_remove = self
            .sqlite
            .query(|connection| -> Result<Box<[(usize, PathBuf)]>, Error> {
                let recordings_to_remove = connection
                    .prepare(indoc!(
                        "
                            SELECT
                                recording_id,
                                path_storage_relative
                            FROM (
                                SELECT
                                    recording_id,
                                    path_storage_relative,
                                    size_bytes,
                                    size_bytes_max,
                                    SUM(size_bytes) OVER(
                                        PARTITION BY
                                            storage_group_id
                                        ORDER BY
                                            (
                                                (detection_level * detection_level_to_second_ratio) -
                                                (CAST(STRFTIME('%s', CURRENT_TIMESTAMP) AS INTEGER) - timestamp_end)
                                            ) DESC
                                        ROWS
                                            UNBOUNDED PRECEDING
                                    ) AS size_bytes_rolling
                                FROM
                                    recordings
                                JOIN
                                    channels USING(channel_id)
                                JOIN
                                    storage_groups USING(storage_group_id)
                            )
                            WHERE
                                size_bytes_rolling > size_bytes_max
                            ORDER BY
                                size_bytes DESC
                        "
                    ))?
                    .query_map([], |row| {
                        let recording_id = row.get_ref_unwrap(0).as_i64()? as usize;
                        let path_storage_relative = PathBuf::from(row.get_ref_unwrap(1).as_str()?);
                        Ok((recording_id, path_storage_relative))
                    })?
                    .collect::<rusqlite::Result<_>>()?;

                Ok(recordings_to_remove)
            })
            .await
            .context("recordings_to_remove query")?;

        // return early, don't remove or clear db
        if recordings_to_remove.is_empty() {
            return Ok(());
        }

        // remove
        let storage_directory_root_path = self.storage_directory_root_path_build();
        let storage_directory_root_path = &storage_directory_root_path;
        for (recording_id, path_storage_relative) in recordings_to_remove.iter() {
            let result: Result<(), Error> = try {
                // remove file
                let path_storage = storage_directory_root_path.join(path_storage_relative);
                fs::remove_file(&path_storage)
                    .await
                    .context("remove_file")?;

                // remove parent directories
                // FIXME: may race with pushing new segment
                remove_all_dir_empty(
                    storage_directory_root_path,
                    path_storage_relative.parent().unwrap(),
                )
                .await
                .context("remove_all_dir_empty")?;
            };
            match result {
                Ok(()) => {}
                Err(error) => {
                    log::error!("{}: cleanup: {:?} (#{})", self, error, recording_id);
                }
            }
        }

        // store information about removed
        self.sqlite
            .query(move |connection| -> Result<(), Error> {
                connection
                    .prepare(indoc!(
                        "
                            DELETE FROM
                                recordings
                            WHERE
                                recording_id IN rarray(?)
                        "
                    ))?
                    .execute(rusqlite::params![Rc::new(
                        recordings_to_remove
                            .iter()
                            .map(|(recording_id, _)| rusqlite::types::Value::from(
                                *recording_id as i64
                            ))
                            .collect::<Vec<_>>()
                    )])?;

                Ok(())
            })
            .await
            .context("removed_recording_ids query")?;

        Ok(())
    }

    const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 5);
    async fn cleanup_loop_run_once(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        loop {
            self.cleanup().await.context("cleanup")?;

            select! {
                () = tokio::time::sleep(Self::CLEANUP_INTERVAL).fuse() => {},
                () = exit_flag => break,
            }
        }

        Ok(Exited)
    }
    async fn cleanup_loop_run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        const ERROR_DELAY: Duration = Duration::from_secs(5);

        loop {
            let error = match self
                .cleanup_loop_run_once(exit_flag.clone())
                .await
                .context("cleanup_loop_run_once")
            {
                Ok(Exited) => break,
                Err(error) => error,
            };
            log::error!("{}: {:?}", self, error);

            select! {
                () = tokio::time::sleep(ERROR_DELAY).fuse() => {},
                () = exit_flag => break,
            }
        }

        Exited
    }

    // path managment
    const STORAGE_DIRECTORY_NAME: &'static str = "devices.soft.surveillance.rtsp_recorder";
    pub fn channel_temporary_directory_path_build(
        &self,
        channel_id: ChannelId,
    ) -> PathBuf {
        self.fs
            .temporary_storage_directory()
            .join(Self::STORAGE_DIRECTORY_NAME)
            .join(self.name.as_str())
            .join(channel_id.to_string())
    }
    fn storage_directory_root_path_build(&self) -> PathBuf {
        self.fs
            .persistent_storage_directory()
            .join(Self::STORAGE_DIRECTORY_NAME)
            .join(self.name.as_str())
    }
    fn segment_storage_relative_path_build(
        channel_id: ChannelId,
        channel_segment: &ChannelSegment,
    ) -> PathBuf {
        // <channel_id>/<year (2020)>/<month (01)>/<day (01)>/<hh>_<mm>_<ss>.<ext>

        let mut path_buf = PathBuf::new();
        path_buf.push(format!("{:0>3}", channel_id));
        path_buf.push(format!("{:0>2}", channel_segment.segment.time_start.year()));
        path_buf.push(format!(
            "{:0>2}",
            channel_segment.segment.time_start.month()
        ));
        path_buf.push(format!("{:0>2}", channel_segment.segment.time_start.day()));
        path_buf.push(format!(
            "{:0>2}_{:0>2}_{:0>2}",
            channel_segment.segment.time_start.hour(),
            channel_segment.segment.time_start.minute(),
            channel_segment.segment.time_start.second(),
        ));
        if let Some(extension) = channel_segment.segment.path.extension() {
            path_buf.set_extension(extension);
        }
        path_buf
    }

    // run procedure
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        // initialize
        let _ = self.initialize(exit_flag.clone()).await;

        // run parts
        // TODO: Check for exit race
        let channel_segment_receiver_runner = self.channel_segment_receiver_run(exit_flag.clone());
        let cleanup_loop_runner = self.cleanup_loop_run(exit_flag.clone());
        let _: (Exited, Exited) = join!(channel_segment_receiver_runner, cleanup_loop_runner);

        Exited
    }
}
#[async_trait]
impl Runnable for Manager<'_> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
impl fmt::Display for Manager<'_> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Manager({})", self.name)
    }
}
