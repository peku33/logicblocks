use bytes::BytesMut;
use chrono::{DateTime, NaiveDateTime, Timelike, Utc};
use failure::{err_msg, format_err, Error};
use futures::future::FutureExt;
use futures::lock::Mutex;
use futures::select;
use futures::stream::StreamExt;
use futures::{Sink, SinkExt};
use std::fs;
use std::fs::Metadata;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use url::Url;

#[cfg(not(target_os = "linux"))]
use crate::stubs::inotify::{EventOwned, Inotify, WatchMask};
#[cfg(target_os = "linux")]
use inotify::{EventOwned, Inotify, WatchMask};

#[derive(Debug)]
pub struct Segment {
    pub path: PathBuf,
    pub metadata: Metadata,
    pub time_start_utc: NaiveDateTime,
    pub time_end_utc: NaiveDateTime,
}

pub struct Recorder {
    rtsp_url: Url,
    segment_time: Duration,
    temporary_storage_directory: PathBuf,
    segment_sender: Mutex<Pin<Box<dyn Sink<Segment, Error = Error> + Send>>>,
}
impl Recorder {
    pub fn new(
        rtsp_url: Url,
        segment_time: Duration,
        temporary_storage_directory: PathBuf,
        segment_sender: Pin<Box<dyn Sink<Segment, Error = Error> + Send>>,
    ) -> Self {
        Self {
            rtsp_url,
            segment_time,
            temporary_storage_directory,
            segment_sender: Mutex::new(segment_sender),
        }
    }

    async fn ffmpeg_run_once(&self) -> Error {
        let mut command = Command::new("/usr/bin/ffmpeg");
        command
            .env_clear()
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Global options
            .args(&["-loglevel", "warning"])
            .arg("-hide_banner")
            .arg("-nostats")
            .arg("-nostdin")
            .args(&["-strict", "experimental"])
            .args(&["-sn", "-dn"])
            .args(&["-threads", "1"])
            // Input options
            .args(&["-f", "rtsp"])
            .args(&["-stimeout", "1000000"])
            .args(&["-i", self.rtsp_url.as_str()])
            // I/O options
            .args(&["-codec", "copy"])
            // Output options
            .args(&["-f", "segment"])
            .args(&["-segment_format", "mp4"])
            .args(&[
                "-segment_time",
                self.segment_time.as_secs().to_string().as_str(),
            ])
            .args(&["-segment_atclocktime", "1"])
            .args(&["-strftime", "1"])
            .args(&["-break_non_keyframes", "1"])
            .args(&["-reset_timestamps", "1"])
            .arg(self.temporary_storage_directory.join("%s.mp4").as_os_str());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => return error.into(),
        };

        let stdout = BufReader::new(match child.stdout.take() {
            Some(stdout) => stdout,
            None => return err_msg("stdout missing"),
        })
        .lines()
        .for_each(async move |item| match item {
            Ok(line) => log::warn!("ffmpeg stdout: {}", line),
            Err(error) => log::error!("error while reading ffmpeg stdout: {}", error),
        });

        let stderr = BufReader::new(match child.stderr.take() {
            Some(stderr) => stderr,
            None => return err_msg("stderr missing"),
        })
        .lines()
        .for_each(async move |item| match item {
            Ok(line) => log::warn!("ffmpeg stderr: {}", line),
            Err(error) => log::error!("error while reading ffmpeg stderr: {}", error),
        });

        select! {
            child_error = child.fuse() => match child_error {
                Ok(exit_code) => format_err!("ffmpeg exited with status code: {}", exit_code),
                Err(error) => format_err!("ffmpeg child error: {}", error),
            },
            _ = stdout.fuse() => err_msg("stdout exited"),
            _ = stderr.fuse() => err_msg("stderr exited"),
        }
    }
    async fn ffmpeg_run(&self) -> Error {
        let error_delay = Duration::from_secs(5);

        loop {
            let ffmpeg_run_once_error = self.ffmpeg_run_once().await;
            log::error!("ffmpeg_run_once error: {}", ffmpeg_run_once_error);
            tokio::time::delay_for(error_delay).await;
        }
    }

    async fn handle_file(
        &self,
        path: PathBuf,
    ) -> Result<(), Error> {
        let file_stem_int = path
            .file_stem()
            .ok_or_else(|| err_msg("missing file_stem"))?
            .to_str()
            .ok_or_else(|| err_msg("failed parsing file_stem"))?
            .parse()?;
        let time_start_utc = NaiveDateTime::from_timestamp(file_stem_int, 0);

        let metadata = fs::metadata(&path)?;
        let time_end_utc = DateTime::<Utc>::from(metadata.modified()?)
            .naive_utc()
            .with_nanosecond(0)
            .unwrap();

        let segment = Segment {
            path,
            metadata,
            time_start_utc,
            time_end_utc,
        };

        self.segment_sender.lock().await.send(segment).await?;

        Ok(())
    }
    async fn handle_inotify_event(
        &self,
        event: Result<EventOwned, std::io::Error>,
    ) -> Result<(), Error> {
        let event = event?;

        let name = event.name.ok_or_else(|| err_msg("missing file name"))?;
        let path = self.temporary_storage_directory.join(name);

        self.handle_file(path).await?;

        Ok(())
    }

    async fn inotify_run_once(&self) -> Error {
        let mut inotify_instance = match Inotify::init() {
            Ok(inotify_instance) => inotify_instance,
            Err(error) => return error.into(),
        };

        if let Err(error) =
            inotify_instance.add_watch(&self.temporary_storage_directory, WatchMask::CLOSE_WRITE)
        {
            return error.into();
        }

        const INOTIFY_BUFFER_SIZE: usize = 1024;
        let mut buffer = BytesMut::with_capacity(INOTIFY_BUFFER_SIZE);
        unsafe { buffer.set_len(INOTIFY_BUFFER_SIZE) };

        let event_stream = match inotify_instance.event_stream(buffer) {
            Ok(event_stream) => event_stream,
            Err(error) => return error.into(),
        };

        event_stream
            .for_each(async move |event| {
                if let Err(error) = self.handle_inotify_event(event).await {
                    log::error!("error in handle_inotify_event(): {}", error);
                }
            })
            .await;

        err_msg("event_stream closed")
    }
    async fn inotify_run(&self) -> Error {
        let error_delay = Duration::from_secs(5);

        loop {
            let inotify_run_once_error = self.inotify_run_once().await;
            log::error!("inotify_run_once error: {}", inotify_run_once_error);
            tokio::time::delay_for(error_delay).await;
        }
    }

    async fn fixer_handle_directory_entry(
        &self,
        dir_entry: tokio::io::Result<fs::DirEntry>,
    ) -> Result<(), Error> {
        let dir_entry = dir_entry?;

        let path = self.temporary_storage_directory.join(dir_entry.file_name());

        let metadata = dir_entry.metadata()?;
        if !metadata.is_file() {
            log::warn!(
                "non-file entry found in fixer_handle_directory_entry: {:?}",
                path
            );
        }

        let now = SystemTime::now();

        // Some platforms does not handle this properly
        // Delete file if either of criteria is met
        let created_old_enough = match metadata.created() {
            Ok(created) => Some(now - 4 * self.segment_time > created),
            Err(_) => None,
        };
        let modified_old_enough = match metadata.modified() {
            Ok(modified) => Some(now - 3 * self.segment_time > modified),
            Err(_) => None,
        };

        let old_enough = match (created_old_enough, modified_old_enough) {
            (Some(true), _) | (_, Some(true)) => true,
            (Some(false), _) | (_, Some(false)) => false,
            (None, None) => {
                log::warn!("unable to determine processing criteria for: {:?}", path);
                false
            }
        };

        if old_enough {
            log::warn!("orphaned file suitable for processing: {:?}", path);
            self.handle_file(path).await?;
        }

        Ok(())
    }
    async fn fixer_run_once(&self) -> Result<(), Error> {
        for entry in fs::read_dir(&self.temporary_storage_directory)? {
            if let Err(error) = self.fixer_handle_directory_entry(entry).await {
                log::error!("error in fixer_handle_directory_entry: {}", error);
            }
        }

        Ok(())
    }
    async fn fixer_run(&self) -> Error {
        let delay = Duration::from_secs(60 * 60);
        loop {
            if let Err(error) = self.fixer_run_once().await {
                log::error!("fixer_run_once error: {}", error);
            }
            tokio::time::delay_for(delay).await;
        }
    }

    async fn init(&self) -> Result<(), Error> {
        if !self.temporary_storage_directory.exists() {
            fs::create_dir_all(&self.temporary_storage_directory)?;
        }

        let temporary_storage_directory_metadata = fs::metadata(&self.temporary_storage_directory)?;

        if !temporary_storage_directory_metadata.is_dir() {
            return Err(err_msg("temporary_storage_directory is not a directory"));
        }

        Ok(())
    }

    pub async fn run(&self) -> Error {
        if let Err(error) = self.init().await {
            return error;
        }
        select! {
            ffmpeg_run_error = self.ffmpeg_run().fuse() => ffmpeg_run_error,
            inotify_run_error = self.inotify_run().fuse() => inotify_run_error,
            fixer_run_error = self.fixer_run().fuse() => fixer_run_error,
        }
    }
}
