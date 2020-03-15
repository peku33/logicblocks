use super::organizer::Organizer;
use super::recorder::{Recorder, Segment};
use crate::modules::fs::Fs;
use crate::modules::{Context, Handle, Module, ModuleFactory};
use crate::util::borrowed_async::DerefAsyncFuture;
use crate::util::select_all_empty::select_all_empty;
use crate::util::tokio_cancelable::ThreadedInfiniteToError;
use failure::Error;
use futures::channel::mpsc;
use futures::future::{BoxFuture, FutureExt};
use futures::lock::Mutex;
use futures::select;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use owning_ref::OwningHandle;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use url::Url;

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct Channel {
    pub id: usize,
    pub rtsp_url: Url,
}

type RecorderOwnedRun = OwningHandle<Box<Recorder>, Box<Mutex<BoxFuture<'static, Error>>>>;

#[derive(Debug)]
struct RecordersSegmentSenderItem {
    channel_id: usize,
    segment: Segment,
}

pub struct RecorderOrganizer {
    channels_update_sender: mpsc::UnboundedSender<HashSet<Channel>>,
    run_handle: ThreadedInfiniteToError,
}
impl RecorderOrganizer {
    pub fn new(
        fs: Handle<Fs>,
        organizer: Handle<Organizer>,
    ) -> Self {
        let (channels_update_sender, channels_update_receiver) = mpsc::unbounded();

        let run_handle = ThreadedInfiniteToError::new(
            "RecorderOrganizer (devices/soft/ipc)".to_owned(),
            Self::run(fs, organizer, channels_update_receiver),
        );

        Self {
            channels_update_sender,
            run_handle,
        }
    }

    pub fn channels_update(
        &self,
        channels: HashSet<Channel>,
    ) {
        self.channels_update_sender
            .unbounded_send(channels)
            .unwrap();
    }

    async fn run(
        fs: Handle<Fs>,
        organizer: Handle<Organizer>,
        channels_update_receiver: mpsc::UnboundedReceiver<HashSet<Channel>>,
    ) -> Error {
        let mut recorder_owned_run_by_channel: HashMap<Channel, RecorderOwnedRun> = HashMap::new();

        let (recorders_segment_item_sender, recorders_segment_item_receiver) = mpsc::unbounded();

        let mut channels_update_receiver = channels_update_receiver.fuse();
        let mut recorders_segment_item_receiver = recorders_segment_item_receiver.fuse();
        let mut organizer_run_future = organizer.run().boxed().fuse();

        loop {
            select! {
                channels_update = channels_update_receiver.next() => {
                    let channels_update = match channels_update {
                        Some(channels_update) => channels_update,
                        None => HashSet::new(),
                    };

                    Self::rebuild_recorder_owned_run_by_channel(
                        channels_update,
                        &mut recorder_owned_run_by_channel,
                        &fs,
                        &recorders_segment_item_sender,
                    );
                },
                recorders_segment = recorders_segment_item_receiver.next() => {
                    let recorders_segment = recorders_segment.unwrap();

                    let handle_recording_result = organizer.handle_recording(
                        recorders_segment.channel_id,
                        &recorders_segment.segment.path,
                        Some(recorders_segment.segment.metadata),
                        recorders_segment.segment.time_start_utc,
                        recorders_segment.segment.time_end_utc
                    ).await;

                    match handle_recording_result {
                        Ok(recording_id) => (),
                        Err(error) => return error,
                    };
                },
                organizer_run_future_error = organizer_run_future => {
                    return organizer_run_future_error;
                },
                (channel_error, _) = select_all_empty(
                    recorder_owned_run_by_channel
                        .values()
                        .map(|recorder_owned_run| DerefAsyncFuture::new(
                            recorder_owned_run.try_lock().unwrap()
                        )),
                ).fuse() => {
                    return channel_error;
                },
            }
        }
    }

    fn rebuild_recorder_owned_run_by_channel(
        mut channels: HashSet<Channel>,
        recorder_owned_run_by_channel: &mut HashMap<Channel, RecorderOwnedRun>,
        fs: &Fs,
        recorders_segment_item_sender: &mpsc::UnboundedSender<RecordersSegmentSenderItem>,
    ) {
        recorder_owned_run_by_channel.retain(|channel, _| channels.contains(channel));
        channels.retain(|channel| !recorder_owned_run_by_channel.contains_key(channel));
        for channel in channels.into_iter() {
            let recorder_owned_run =
                Self::build_recorder_owned_run(&channel, fs, recorders_segment_item_sender);
            let insert_result = recorder_owned_run_by_channel.insert(channel, recorder_owned_run);
            assert!(insert_result.is_none());
        }
    }

    fn build_recorder_owned_run(
        channel: &Channel,
        fs: &Fs,
        recorders_segment_item_sender: &mpsc::UnboundedSender<RecordersSegmentSenderItem>,
    ) -> RecorderOwnedRun {
        RecorderOwnedRun::new_with_fn(
            Box::new(Self::build_recorder(
                channel,
                fs,
                recorders_segment_item_sender,
            )),
            unsafe { |recorder_ptr| Box::new(Mutex::new((*recorder_ptr).run().boxed())) },
        )
    }

    fn build_recorder(
        channel: &Channel,
        fs: &Fs,
        recorders_segment_item_sender: &mpsc::UnboundedSender<RecordersSegmentSenderItem>,
    ) -> Recorder {
        let duration = Duration::from_secs(60);

        let temporary_storage_directory = fs
            .temporary_storage_directory()
            .join("devices_soft_ipc_organizer_recorder")
            .join(channel.id.to_string());

        let channel_id = channel.id;
        let segment_sender = Box::pin(recorders_segment_item_sender.clone().with(
            async move |segment| {
                Ok(RecordersSegmentSenderItem {
                    channel_id,
                    segment,
                })
            },
        ));

        Recorder::new(
            channel.rtsp_url.clone(),
            duration,
            temporary_storage_directory,
            segment_sender,
        )
    }
}
impl Drop for RecorderOrganizer {
    fn drop(&mut self) {}
}

impl Module for RecorderOrganizer {}
impl ModuleFactory for RecorderOrganizer {
    fn spawn(context: &Context) -> Self {
        Self::new(context.get::<Fs>(), context.get::<Organizer>())
    }
}
