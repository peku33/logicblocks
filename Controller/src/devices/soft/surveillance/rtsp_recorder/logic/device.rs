use super::super::hardware::channel::Channel;
use crate::{
    datatypes::{ipc_rtsp_url::IpcRtspUrl, ratio::Ratio},
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
};
use async_trait::async_trait;
use maplit::hashmap;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Device<'c> {
    channel: &'c Channel,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_rtsp_url: signal::state_target_last::Signal<IpcRtspUrl>,
    signal_detection_level: signal::state_target_last::Signal<Ratio>,
}
impl<'c> Device<'c> {
    pub fn new(channel: &'c Channel) -> Self {
        channel.rtsp_url_set(None);
        channel.detection_level_set(None);

        Self {
            channel,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_rtsp_url: signal::state_target_last::Signal::<IpcRtspUrl>::new(),
            signal_detection_level: signal::state_target_last::Signal::<Ratio>::new(),
        }
    }
}
impl<'c> devices::Device for Device<'c> {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/surveillance/rtsp_recorder/channel")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
}
#[async_trait]
impl<'c> Runnable for Device<'c> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        exit_flag.await;
        Exited
    }
}
impl<'c> signals::Device for Device<'c> {
    fn signal_targets_changed_wake(&self) {
        if let Some(rtsp_url) = self.signal_rtsp_url.take_pending() {
            self.channel.rtsp_url_set(rtsp_url);
        }
        if let Some(detection_level) = self.signal_detection_level.take_pending() {
            self.channel.detection_level_set(detection_level);
        }
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_rtsp_url as &dyn signal::Base,
            1 => &self.signal_detection_level as &dyn signal::Base,
        }
    }
}
