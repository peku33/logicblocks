use super::recorder::{Recorder, Segment};
use crate::{
    datatypes::{ipc_rtsp_url::IpcRtspUrl, ratio::Ratio},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        atomic_cell::{AtomicCell, AtomicCellLease},
        runtime::{Exited, Runnable},
    },
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use futures::{
    channel::mpsc,
    future::FutureExt,
    join, select,
    stream::{StreamExt, TryStreamExt},
};
use parking_lot::Mutex;
use std::{fmt, path::PathBuf, time::Duration};
use tokio::fs;

#[derive(Debug)]
pub struct ChannelSegment {
    pub segment: Segment,
    pub detection_level: Ratio,
}

#[derive(Debug)]
struct DetectionLevelTracker {
    current: Option<Ratio>,
    segment_max: Option<Ratio>,
}
impl DetectionLevelTracker {
    pub fn new() -> Self {
        Self {
            current: None,
            segment_max: None,
        }
    }

    pub fn current_set(
        &mut self,
        current: Option<Ratio>,
    ) {
        self.current = current;

        match (self.current.as_mut(), self.segment_max.as_mut()) {
            (Some(current), Some(segment_max)) => {
                if current > segment_max {
                    *segment_max = *current;
                }
            }
            (Some(current), None) => {
                self.segment_max = Some(*current);
            }
            _ => {}
        };
    }

    pub fn segment_finalize(&mut self) -> Option<Ratio> {
        let segment_max = self.segment_max;
        self.segment_max = self.current;
        segment_max
    }
}

#[derive(Debug)]
pub struct Channel {
    detection_level_threshold: Ratio,
    detection_level_tracker: Mutex<DetectionLevelTracker>,

    recorder_segment_receiver: AtomicCell<mpsc::UnboundedReceiver<Segment>>,
    recorder: Recorder,

    channel_segment_sender: mpsc::UnboundedSender<ChannelSegment>,
    channel_segment_receiver: AtomicCell<mpsc::UnboundedReceiver<ChannelSegment>>,
}
impl Channel {
    pub fn new(
        rtsp_url: Option<IpcRtspUrl>,
        segment_time: Duration,
        temporary_storage_directory: PathBuf,
        detection_level_threshold: Ratio,
    ) -> Self {
        let detection_level_tracker = DetectionLevelTracker::new();
        let detection_level_tracker = Mutex::new(detection_level_tracker);

        let (recorder_segment_sender, recorder_segment_receiver) = mpsc::unbounded::<Segment>();
        let recorder_segment_receiver = AtomicCell::new(recorder_segment_receiver);

        let recorder = Recorder::new(
            rtsp_url,
            segment_time,
            temporary_storage_directory,
            recorder_segment_sender,
        );

        let (channel_segment_sender, channel_segment_receiver) =
            mpsc::unbounded::<ChannelSegment>();
        let channel_segment_receiver = AtomicCell::new(channel_segment_receiver);

        Self {
            detection_level_threshold,
            detection_level_tracker,

            recorder_segment_receiver,
            recorder,

            channel_segment_sender,
            channel_segment_receiver,
        }
    }

    pub fn channel_segment_receiver_lease(
        &self
    ) -> AtomicCellLease<mpsc::UnboundedReceiver<ChannelSegment>> {
        self.channel_segment_receiver.lease()
    }

    // state setters
    pub fn rtsp_url_set(
        &self,
        rtsp_url: Option<IpcRtspUrl>,
    ) {
        self.recorder.rtsp_url_set(rtsp_url)
    }
    pub fn detection_level_set(
        &self,
        detection_level: Option<Ratio>,
    ) {
        self.detection_level_tracker
            .lock()
            .current_set(detection_level);
    }

    // segment handler runner
    async fn channel_segment_forward(
        &self,
        segment: Segment,
    ) -> Result<(), Error> {
        let detection_level = self
            .detection_level_tracker
            .lock()
            .segment_finalize()
            .unwrap_or_else(Ratio::epsilon);

        if detection_level >= self.detection_level_threshold {
            let channel_segment = ChannelSegment {
                segment,
                detection_level,
            };

            self.channel_segment_sender
                .unbounded_send(channel_segment)
                .unwrap();
        } else {
            fs::remove_file(segment.path).await.context("remove_file")?;
        }

        Ok(())
    }
    async fn channel_segment_forwarder_run_once(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        self.recorder_segment_receiver
            .lease()
            .by_ref()
            .stream_take_until_exhausted(exit_flag)
            .map(Ok)
            .try_for_each(async move |segment| -> Result<(), Error> {
                self.channel_segment_forward(segment)
                    .await
                    .context("channel_segment_forward")?;

                Ok(())
            })
            .await
            .context("recorder_segment_receiver")?;

        Ok(Exited)
    }
    async fn channel_segment_forwarder_run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        const ERROR_DELAY: Duration = Duration::from_secs(5);

        loop {
            let error = match self
                .channel_segment_forwarder_run_once(exit_flag.clone())
                .await
                .context("channel_segment_forwarder_run_once")
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

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        let recorder_exit_flag = exit_flag;
        let (
            channel_segment_forwarder_exit_flag_sender,
            channel_segment_forwarder_exit_flag_receiver,
        ) = async_flag::pair();

        let recorder_runner = self
            .recorder
            .run(recorder_exit_flag)
            .then(async move |_: Exited| {
                channel_segment_forwarder_exit_flag_sender.signal();
                Exited
            });

        let channel_segment_forwarder_runner = self
            .channel_segment_forwarder_run(channel_segment_forwarder_exit_flag_receiver)
            .then(async move |_: Exited| Exited);

        let _: (Exited, Exited) = join!(recorder_runner, channel_segment_forwarder_runner);

        Exited
    }
}
#[async_trait]
impl Runnable for Channel {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
impl fmt::Display for Channel {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Channel({})", self.recorder)
    }
}

#[cfg(test)]
mod tests_detection_level_tracker {
    use super::DetectionLevelTracker;
    use crate::datatypes::ratio::Ratio;

    #[test]
    fn test_nop() {
        let mut dlt = DetectionLevelTracker::new();
        assert_eq!(dlt.segment_finalize(), None);
        dlt.current_set(None);
        assert_eq!(dlt.segment_finalize(), None);
    }
    #[test]
    fn test_sequence_1() {
        let mut dlt = DetectionLevelTracker::new();
        dlt.current_set(None);
        assert_eq!(dlt.segment_finalize(), None);
        dlt.current_set(Some(Ratio::full()));
        assert_eq!(dlt.segment_finalize(), Some(Ratio::full()));
    }
    #[test]
    fn test_sequence_2() {
        let mut dlt = DetectionLevelTracker::new();
        dlt.current_set(Some(Ratio::full()));
        dlt.current_set(Some(Ratio::zero()));
        assert_eq!(dlt.segment_finalize(), Some(Ratio::full()));
        dlt.current_set(Some(Ratio::epsilon()));
        dlt.current_set(Some(Ratio::zero()));
        assert_eq!(dlt.segment_finalize(), Some(Ratio::epsilon()));
        assert_eq!(dlt.segment_finalize(), Some(Ratio::zero()));
        assert_eq!(dlt.segment_finalize(), Some(Ratio::zero()));
        dlt.current_set(None);
        assert_eq!(dlt.segment_finalize(), Some(Ratio::zero()));
        assert_eq!(dlt.segment_finalize(), None);
    }
}
