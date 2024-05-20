use crate::{
    datatypes::ipc_rtsp_url::IpcRtspUrl,
    util::{
        anyhow_multiple_error::AnyhowMultipleError,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use anyhow::{anyhow, ensure, Context, Error};
use async_trait::async_trait;
use bytes::BytesMut;
use chrono::{DateTime, NaiveDateTime, Timelike, Utc};
use futures::{
    channel::mpsc,
    future::{Either, FutureExt},
    join, pin_mut, select,
    stream::{StreamExt, TryStreamExt},
};
use std::{
    fmt,
    fs::Metadata,
    io,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime},
};
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::watch,
};

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

#[derive(Debug)]
pub struct Recorder {
    rtsp_url_sender: watch::Sender<Option<IpcRtspUrl>>,
    rtsp_url_receiver: watch::Receiver<Option<IpcRtspUrl>>,

    segment_time: Duration,

    temporary_storage_directory: PathBuf,

    segment_sender: mpsc::UnboundedSender<Segment>,
}
impl Recorder {
    pub fn new(
        rtsp_url: Option<IpcRtspUrl>,
        segment_time: Duration,
        temporary_storage_directory: PathBuf,
        segment_sender: mpsc::UnboundedSender<Segment>,
    ) -> Self {
        let (rtsp_url_sender, rtsp_url_receiver) = watch::channel(rtsp_url);

        Self {
            rtsp_url_sender,
            rtsp_url_receiver,

            segment_time,

            temporary_storage_directory,

            segment_sender,
        }
    }

    pub fn rtsp_url_set(
        &self,
        rtsp_url: Option<IpcRtspUrl>,
    ) {
        self.rtsp_url_sender.send(rtsp_url).unwrap();
    }

    fn handle_segment(
        &self,
        segment: Segment,
    ) -> Result<(), Error> {
        self.segment_sender
            .unbounded_send(segment)
            .context("segment_sender")?;

        Ok(())
    }
    async fn handle_file(
        &self,
        path_relative: &Path, // relative to temporary_storage_directory
    ) -> Result<(), Error> {
        let path = self.temporary_storage_directory.join(path_relative);

        let file_stem_int = path
            .file_stem()
            .ok_or_else(|| anyhow!("missing file_stem"))
            .context("file_stem_int")?
            .to_str()
            .ok_or_else(|| anyhow!("failed parsing file_stem"))
            .context("file_stem_int")?
            .parse()
            .context("file_stem_int")?;
        let time_start_utc =
            NaiveDateTime::from_timestamp_opt(file_stem_int, 0).context("from_timestamp_opt")?;

        let metadata = fs::metadata(&path).await.context("metadata")?;
        let time_end_utc = DateTime::<Utc>::from(metadata.modified().context("modified")?)
            .naive_utc()
            .with_nanosecond(0)
            .unwrap();

        let segment = Segment {
            path,
            metadata,
            time_start_utc,
            time_end_utc,
        };

        self.handle_segment(segment).context("handle_segment")?;

        Ok(())
    }

    fn ffmpeg_build_command(
        &self,
        rtsp_url: &IpcRtspUrl,
    ) -> Command {
        let mut command = Command::new("/usr/bin/ffmpeg");
        command
            .env_clear()
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // global options
            .args(["-loglevel", "error"])
            .arg("-hide_banner")
            .arg("-nostats")
            .arg("-nostdin")
            .args(["-strict", "experimental"])
            .args(["-sn", "-dn"])
            .args(["-threads", "1"])
            // input options
            .args(["-f", "rtsp"])
            .args(["-rtsp_transport", "tcp"])
            .args(["-use_wallclock_as_timestamps", "1"])
            .args(["-timeout", "1000000"])
            .args(["-i", rtsp_url.to_string().as_str()])
            // i/o options
            .args(["-codec", "copy"])
            // output options
            .args(["-f", "segment"])
            .args(["-segment_format", "matroska"])
            .args([
                "-segment_time",
                self.segment_time.as_secs().to_string().as_str(),
            ])
            .args(["-segment_atclocktime", "1"])
            .args(["-strftime", "1"])
            .args(["-break_non_keyframes", "1"])
            .args(["-reset_timestamps", "1"])
            .args(["-avoid_negative_ts", "1"])
            .arg(self.temporary_storage_directory.join("%s.mkv").as_os_str());
        command
    }
    async fn ffmpeg_run_once(
        &self,
        rtsp_url: &IpcRtspUrl,
        mut exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        // build command
        let mut command = self.ffmpeg_build_command(rtsp_url);

        // start process
        let mut child = command.spawn().context("spawn")?;

        // attach stdin/stderr streams
        let stdout = tokio_stream::wrappers::LinesStream::new(
            BufReader::new(child.stdout.take().unwrap()).lines(),
        )
        .for_each(|item| async move {
            match item.context("stdout") {
                Ok(line) => log::warn!("{}: ffmpeg stdout: {}", self, line),
                Err(error) => {
                    log::error!("{}: error while reading ffmpeg stdout: {:?}", self, error)
                }
            }
        });
        pin_mut!(stdout);
        let mut stdout = stdout.fuse();

        let stderr = tokio_stream::wrappers::LinesStream::new(
            BufReader::new(child.stderr.take().unwrap()).lines(),
        )
        .for_each(|item| async move {
            match item.context("stderr") {
                Ok(line) => log::warn!("{}: ffmpeg stderr: {}", self, line),
                Err(error) => {
                    log::error!("{}: error while reading ffmpeg stderr: {:?}", self, error)
                }
            }
        });
        pin_mut!(stderr);
        let mut stderr = stderr.fuse();

        let mut pid = Some(child.id().unwrap());

        let child_exit_future = child.wait();
        pin_mut!(child_exit_future);
        let mut child_exit_future = child_exit_future.fuse();

        // run until error or exit flag
        let result = select! {
            child_exit = child_exit_future => {
                pid.take();

                match child_exit {
                    Ok(exit_code) => Err(anyhow!("ffmpeg exited with status code: {}", exit_code)),
                    Err(error) => Err(anyhow!("ffmpeg child error: {:?}", error)),
                }
            },
            _ = stdout => {
                // pid.take();

                Err(anyhow!("stdout exited"))
            },
            _ = stderr => {
                // pid.take();

                Err(anyhow!("stderr exited"))
            },
            () = exit_flag => Ok(Exited),
        };

        // finalize
        #[cfg(target_os = "linux")]
        if let Some(pid) = pid {
            unsafe { libc::kill(pid as i32, libc::SIGINT) };
        }

        // wait for process exit
        child_exit_future.await.unwrap();

        result
    }
    async fn ffmpeg_run(
        &self,
        rtsp_url: &IpcRtspUrl,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        const ERROR_DELAY: Duration = Duration::from_secs(5);

        loop {
            let error = match self
                .ffmpeg_run_once(rtsp_url, exit_flag.clone())
                .await
                .context("ffmpeg_run_once")
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
    async fn ffmpeg_or_nop_run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let mut rtsp_url_receiver = self.rtsp_url_receiver.clone();

        let mut exit_flag_signaled = false;
        while !exit_flag_signaled {
            let rtsp_url = rtsp_url_receiver.borrow().clone();

            let (ffmpeg_or_nop_run_exit_flag_sender, ffmpeg_or_nop_run_exit_flag_receiver) =
                async_flag::pair();

            // either use a recorder or empty exit flag future
            let ffmpeg_or_nop_runner = match rtsp_url.as_ref() {
                Some(rtsp_url) => {
                    // recorder is set, so wait until it exits correctly
                    Either::Left(self.ffmpeg_run(rtsp_url, ffmpeg_or_nop_run_exit_flag_receiver))
                }
                None => {
                    // recorder is not set, so just wait for the exit flag
                    Either::Right(ffmpeg_or_nop_run_exit_flag_receiver.map(|()| Exited))
                }
            };
            pin_mut!(ffmpeg_or_nop_runner);
            let mut ffmpeg_or_nop_runner = ffmpeg_or_nop_runner.fuse();

            select! {
                result = rtsp_url_receiver.changed().fuse() => {
                    // parent requested channel reload
                    let _: () = result.unwrap();
                    ffmpeg_or_nop_run_exit_flag_sender.signal();
                },
                () = exit_flag => {
                    // parent requested exit, forward to child and exit
                    ffmpeg_or_nop_run_exit_flag_sender.signal();
                    exit_flag_signaled = true;
                },
                Exited = ffmpeg_or_nop_runner => {
                    // this should never happen, because we trigger exit flag in upper cases and never come back
                    panic!("ffmpeg_or_nop_runner yielded")
                },
            }

            // finalize recorder
            let _: Exited = ffmpeg_or_nop_runner.await;
        }

        Exited
    }

    async fn inotify_handle_event(
        &self,
        event: Result<EventOwned, io::Error>,
    ) -> Result<(), Error> {
        let event = event.context("event")?;

        let name = event
            .name
            .ok_or_else(|| anyhow!("missing file name"))
            .context("name")?;

        self.handle_file(Path::new(&name))
            .await
            .context("handle_file")?;

        Ok(())
    }
    async fn inotify_run_once(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        let mut inotify_instance = Inotify::init().context("inotify_instance")?;
        inotify_instance
            .add_watch(&self.temporary_storage_directory, WatchMask::CLOSE_WRITE)
            .context("add_watch")?;

        const INOTIFY_BUFFER_SIZE: usize = 1024;
        let mut buffer = BytesMut::with_capacity(INOTIFY_BUFFER_SIZE);
        unsafe { buffer.set_len(INOTIFY_BUFFER_SIZE) };

        let error_stream = inotify_instance
            .event_stream(buffer)
            .context("event_stream")?
            .filter_map(|event| async move {
                match self
                    .inotify_handle_event(event)
                    .await
                    .context("inotify_handle_event")
                {
                    Ok(()) => None,
                    Err(error) => Some(error),
                }
            });
        pin_mut!(error_stream);

        select! {
            error = error_stream.next().fuse() => match error {
                Some(error) => Err(error),
                None => Err(anyhow!("error_stream closed")),
            },
            () = exit_flag => Ok(Exited),
        }
    }
    async fn inotify_run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        const ERROR_DELAY: Duration = Duration::from_secs(5);

        loop {
            let error = match self
                .inotify_run_once(exit_flag.clone())
                .await
                .context("inotify_run_once")
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

    async fn fixer_handle_directory_entry(
        &self,
        entry: fs::DirEntry,
    ) -> Result<(), Error> {
        let metadata = entry.metadata().await.context("metadata")?;
        if !metadata.is_file() {
            log::warn!(
                "{}: non-file entry found in fixer_handle_directory_entry: {:?}",
                self,
                entry.path()
            );
        }

        let now = SystemTime::now();

        // some platforms does not handle this properly
        // delete file if either of criteria is met
        let created_old_enough = match metadata.created().context("created") {
            Ok(created) => Some(now - 4 * self.segment_time > created),
            Err(_) => None,
        };
        let modified_old_enough = match metadata.modified().context("modified") {
            Ok(modified) => Some(now - 3 * self.segment_time > modified),
            Err(_) => None,
        };

        let old_enough = match (created_old_enough, modified_old_enough) {
            (Some(true), _) | (_, Some(true)) => true,
            (Some(false), _) | (_, Some(false)) => false,
            (None, None) => {
                log::warn!(
                    "{}: unable to determine processing criteria for: {:?}",
                    self,
                    entry.path()
                );
                false
            }
        };

        if old_enough {
            log::warn!(
                "{}: orphaned file suitable for processing: {:?}",
                self,
                entry.path()
            );
            self.handle_file(&entry.path())
                .await
                .context("handle_file")?;
        }

        Ok(())
    }
    async fn fixer_run_once(&self) -> Result<(), Error> {
        let errors = tokio_stream::wrappers::ReadDirStream::new(
            fs::read_dir(&self.temporary_storage_directory)
                .await
                .context("read_dir")?,
        )
        .err_into::<Error>()
        .and_then(|entry| async move {
            self.fixer_handle_directory_entry(entry)
                .await
                .context("fixer_handle_directory_entry")?;

            Ok(())
        })
        .filter_map(|result| async move { result.err() })
        .collect::<Vec<_>>()
        .await;

        if !errors.is_empty() {
            return Err(AnyhowMultipleError::new(errors.into_boxed_slice()).into());
        }

        Ok(())
    }
    async fn fixer_run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        const INTERVAL: Duration = Duration::from_secs(60 * 60);

        loop {
            select! {
                () = tokio::time::sleep(INTERVAL).fuse() => {},
                () = exit_flag => break,
            }

            if let Err(error) = self.fixer_run_once().await.context("fixer_run_once") {
                log::error!("{}: {:?}", self, error);
            }
        }

        Exited
    }

    async fn temporary_storage_directory_prepare(&self) -> Result<(), Error> {
        if !self.temporary_storage_directory.exists() {
            fs::create_dir_all(&self.temporary_storage_directory)
                .await
                .context("create_dir_all")?;
        }

        let temporary_storage_directory_metadata = fs::metadata(&self.temporary_storage_directory)
            .await
            .context("metadata")?;
        ensure!(
            temporary_storage_directory_metadata.is_dir(),
            "temporary_storage_directory is not a directory"
        );

        Ok(())
    }
    async fn temporary_storage_directory_cleanup(&self) -> Result<(), Error> {
        let errors = tokio_stream::wrappers::ReadDirStream::new(
            fs::read_dir(&self.temporary_storage_directory)
                .await
                .context("read_dir")?,
        )
        .err_into::<Error>()
        .and_then(|entry| async move {
            self.handle_file(&entry.path())
                .await
                .context("handle_file")?;

            Ok(())
        })
        .filter_map(|result| async move { result.err() })
        .collect::<Vec<_>>()
        .await;

        if !errors.is_empty() {
            return Err(AnyhowMultipleError::new(errors.into_boxed_slice()).into());
        }

        Ok(())
    }

    async fn initialize_once(&self) -> Result<(), Error> {
        // create directories
        self.temporary_storage_directory_prepare()
            .await
            .context("temporary_storage_directory_prepare")?;

        // handle currently available files
        self.temporary_storage_directory_cleanup()
            .await
            .context("temporary_storage_directory_cleanup")?;

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

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        // initialize
        let _ = self.initialize(exit_flag.clone()).await;

        // run parts
        let fixer_exit_flag_receiver = exit_flag;
        let (ffmpeg_or_nop_run_exit_flag_sender, ffmpeg_or_nop_run_exit_flag_receiver) =
            async_flag::pair();
        let (inotify_exit_flag_sender, inotify_exit_flag_receiver) = async_flag::pair();

        let fixer_runner = self
            .fixer_run(fixer_exit_flag_receiver)
            .inspect(|_: &Exited| {
                ffmpeg_or_nop_run_exit_flag_sender.signal();
            });
        let ffmpeg_or_nop_runner = self
            .ffmpeg_or_nop_run(ffmpeg_or_nop_run_exit_flag_receiver)
            .inspect(|_: &Exited| {
                inotify_exit_flag_sender.signal();
            });
        let inotify_runner = self.inotify_run(inotify_exit_flag_receiver);

        let _: (Exited, Exited, Exited) = join!(fixer_runner, ffmpeg_or_nop_runner, inotify_runner);

        Exited
    }
}
impl fmt::Display for Recorder {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Recorder ({:?})", self.rtsp_url_receiver.borrow())
    }
}
#[async_trait]
impl Runnable for Recorder {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
