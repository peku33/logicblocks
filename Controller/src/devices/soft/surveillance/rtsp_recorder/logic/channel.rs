use super::super::hardware::channel::Channel;
use crate::{
    datatypes::{ipc_rtsp_url::IpcRtspUrl, ratio::Ratio},
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use maplit::hashmap;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Device<'c> {
    channel: &'c Channel,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signal_rtsp_url: signal::state_target_last::Signal<IpcRtspUrl>,
    signal_detection_level: signal::state_target_last::Signal<Ratio>,
}
impl<'c> Device<'c> {
    pub fn new(channel: &'c Channel) -> Self {
        Self {
            channel,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signal_rtsp_url: signal::state_target_last::Signal::<IpcRtspUrl>::new(),
            signal_detection_level: signal::state_target_last::Signal::<Ratio>::new(),
        }
    }

    fn signals_targets_changed(&self) {
        if let Some(rtsp_url) = self.signal_rtsp_url.take_pending() {
            self.channel.rtsp_url_set(rtsp_url);
        }
        if let Some(detection_level) = self.signal_detection_level.take_pending() {
            self.channel.detection_level_set(detection_level);
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.channel.rtsp_url_set(None);
        self.channel.detection_level_set(None);

        self.signals_targets_changed_waker
            .stream()
            .stream_take_until_exhausted(exit_flag)
            .for_each(async |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl devices::Device for Device<'_> {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/surveillance/rtsp_recorder/channel")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
}

#[async_trait]
impl Runnable for Device<'_> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalIdentifier {
    RtspUrl,
    DetectionLevel,
}
impl signals::Identifier for SignalIdentifier {}
impl signals::Device for Device<'_> {
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        Some(&self.signals_targets_changed_waker)
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        None
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::RtspUrl => &self.signal_rtsp_url as &dyn signal::Base,
            SignalIdentifier::DetectionLevel => &self.signal_detection_level as &dyn signal::Base,
        }
    }
}
