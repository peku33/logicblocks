use super::hardware::{api, configurator, event_stream};
use crate::{
    datatypes::ipc_rtsp_url::IpcRtspUrl,
    devices::{
        self,
        soft::surveillance::snapshot::logic_device_inner::{
            Manager as SnapshotManager, Runner as SnapshotRunner,
        },
        GuiSummaryProvider,
    },
    signals::{self, signal},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
    web,
    web::uri_cursor,
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{
    future::{BoxFuture, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use http::uri::Authority;
use maplit::hashmap;
use parking_lot::RwLock;
use serde::Serialize;
use std::{borrow::Cow, time::Duration};

const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum HardwareConfiguration {
    Full {
        hardware_configuration: configurator::Configuration,
    },
    Skip {
        shared_user_login: String,
        shared_user_password: String,
    },
}
#[derive(Debug)]
pub struct Configuration {
    pub host: Authority,
    pub admin_password: String,
    pub hardware_configuration: HardwareConfiguration,
}

#[derive(Serialize, Clone, Debug)]
pub struct RtspUrls {
    main: IpcRtspUrl,
    sub: IpcRtspUrl,
}

#[derive(Serialize, Copy, Clone, Default, Debug)]
pub struct Events {
    camera_failure: bool,
    video_loss: bool,
    tampering_detection: bool,
    motion_detection: bool,
    line_detection: bool,
    field_detection: bool,
}
impl Events {
    pub fn from_event_stream_events(hardware_events: &event_stream::Events) -> Self {
        Self {
            camera_failure: hardware_events.contains(&event_stream::Event::CameraFailure),
            video_loss: hardware_events.contains(&event_stream::Event::VideoLoss),
            tampering_detection: hardware_events.contains(&event_stream::Event::TamperingDetection),
            motion_detection: hardware_events.contains(&event_stream::Event::MotionDetection),
            line_detection: hardware_events.contains(&event_stream::Event::LineDetection),
            field_detection: hardware_events.contains(&event_stream::Event::FieldDetection),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "state")]
pub enum DeviceState {
    Initializing,
    Running {
        snapshot_updated: Option<DateTime<Utc>>,
        rtsp_urls: RtspUrls,
        events: Events,
    },
    Error,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    device_state: RwLock<DeviceState>,
    snapshot_manager: SnapshotManager,

    gui_summary_waker: waker_stream::mpmc::Sender,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_rtsp_url_main: signal::state_source::Signal<IpcRtspUrl>,
    signal_rtsp_url_sub: signal::state_source::Signal<IpcRtspUrl>,
    signal_event_camera_failure: signal::state_source::Signal<bool>,
    signal_event_video_loss: signal::state_source::Signal<bool>,
    signal_event_tampering_detection: signal::state_source::Signal<bool>,
    signal_event_motion_detection: signal::state_source::Signal<bool>,
    signal_event_line_detection: signal::state_source::Signal<bool>,
    signal_event_field_detection: signal::state_source::Signal<bool>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            device_state: RwLock::new(DeviceState::Initializing),
            snapshot_manager: SnapshotManager::new(),

            gui_summary_waker: waker_stream::mpmc::Sender::new(),

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_rtsp_url_main: signal::state_source::Signal::<IpcRtspUrl>::new(None),
            signal_rtsp_url_sub: signal::state_source::Signal::<IpcRtspUrl>::new(None),
            signal_event_camera_failure: signal::state_source::Signal::<bool>::new(None),
            signal_event_video_loss: signal::state_source::Signal::<bool>::new(None),
            signal_event_tampering_detection: signal::state_source::Signal::<bool>::new(None),
            signal_event_motion_detection: signal::state_source::Signal::<bool>::new(None),
            signal_event_line_detection: signal::state_source::Signal::<bool>::new(None),
            signal_event_field_detection: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn snapshot_updated_handle(&self) {
        match &mut *self.device_state.write() {
            DeviceState::Running {
                snapshot_updated, ..
            } => {
                snapshot_updated.replace(Utc::now());
            }
            _ => panic!("snapshot_updated_handle can be called only when device is running"),
        }
        self.gui_summary_waker.wake();
    }
    fn events_handle(
        &self,
        events: Events,
    ) {
        match &mut *self.device_state.write() {
            DeviceState::Running {
                events: state_events,
                ..
            } => *state_events = events,
            _ => panic!("events_handle can be called only when device is running"),
        }
        self.gui_summary_waker.wake();

        let mut signals_changed = false;
        signals_changed |= self
            .signal_event_camera_failure
            .set_one(Some(events.camera_failure));
        signals_changed |= self // break
            .signal_event_video_loss
            .set_one(Some(events.video_loss));
        signals_changed |= self
            .signal_event_tampering_detection
            .set_one(Some(events.tampering_detection));
        signals_changed |= self
            .signal_event_motion_detection
            .set_one(Some(events.motion_detection));
        signals_changed |= self
            .signal_event_line_detection
            .set_one(Some(events.line_detection));
        signals_changed |= self
            .signal_event_field_detection
            .set_one(Some(events.field_detection));
        if signals_changed {
            self.signal_sources_changed_waker.wake();
        }
    }

    fn failed(&self) {
        *self.device_state.write() = DeviceState::Error;
        self.gui_summary_waker.wake();

        self.snapshot_manager.image_unset();

        let _ = self.signal_rtsp_url_main.set_one(None);
        let _ = self.signal_rtsp_url_sub.set_one(None);
        let _ = self.signal_event_camera_failure.set_one(None);
        let _ = self.signal_event_video_loss.set_one(None);
        let _ = self.signal_event_tampering_detection.set_one(None);
        let _ = self.signal_event_motion_detection.set_one(None);
        let _ = self.signal_event_line_detection.set_one(None);
        let _ = self.signal_event_field_detection.set_one(None);
        self.signal_sources_changed_waker.wake();
    }

    async fn run_once(&self) -> Result<!, Error> {
        *self.device_state.write() = DeviceState::Initializing;
        self.gui_summary_waker.wake();

        // Build client
        let api = api::Api::new(
            self.configuration.host.clone(),
            self.configuration.admin_password.clone(),
        );

        // Check device
        api.validate_basic_device_info()
            .await
            .context("validate_basic_device_info")?;

        // Set device configuration
        // Get rtsp data based on configuration type
        let (shared_user_login, shared_user_password) =
            match &self.configuration.hardware_configuration {
                HardwareConfiguration::Full {
                    hardware_configuration,
                } => {
                    let mut configurator = configurator::Configurator::new(&api);
                    configurator
                        .configure(hardware_configuration.clone())
                        .await
                        .context("configure")?;

                    (
                        configurator::Configurator::SHARED_USER_LOGIN,
                        &hardware_configuration.shared_user_password,
                    )
                }
                HardwareConfiguration::Skip {
                    shared_user_login,
                    shared_user_password,
                } => (shared_user_login.as_str(), shared_user_password),
            };

        // Set device video URLs
        let rtsp_urls = RtspUrls {
            main: IpcRtspUrl::new(api.rtsp_url_build(
                shared_user_login,
                shared_user_password,
                api::VideoStream::Main,
            )),
            sub: IpcRtspUrl::new(api.rtsp_url_build(
                shared_user_login,
                shared_user_password,
                api::VideoStream::Sub,
            )),
        };

        // Attach event manager
        let events_stream_manager = event_stream::Manager::new(&api);
        let mut events_stream_manager_receiver =
            tokio_stream::wrappers::WatchStream::new(events_stream_manager.receiver());

        let events_stream_manager_runner = events_stream_manager.run_once();
        pin_mut!(events_stream_manager_runner);
        let mut events_stream_manager_runner = events_stream_manager_runner.fuse();

        let events_stream_manager_receiver_runner = events_stream_manager_receiver
            .by_ref()
            .for_each(async move |hardware_events| {
                let events = Events::from_event_stream_events(&hardware_events);
                self.events_handle(events);
            });
        pin_mut!(events_stream_manager_receiver_runner);
        let mut events_stream_manager_receiver_runner =
            events_stream_manager_receiver_runner.fuse();

        // Attach snapshot manager
        let snapshot_runner = SnapshotRunner::new(
            &self.snapshot_manager,
            || api.snapshot(),
            || self.snapshot_updated_handle(),
            SNAPSHOT_INTERVAL,
        );
        let snapshot_runner_runner = snapshot_runner.run_once();
        pin_mut!(snapshot_runner_runner);
        let mut snapshot_runner_runner = snapshot_runner_runner.fuse();

        // Mark device as ready
        *self.device_state.write() = DeviceState::Running {
            snapshot_updated: None,
            rtsp_urls: rtsp_urls.clone(),
            events: Events::default(),
        };
        self.gui_summary_waker.wake();

        // Set initial signal values
        let _ = self.signal_rtsp_url_main.set_one(Some(rtsp_urls.main));
        let _ = self.signal_rtsp_url_sub.set_one(Some(rtsp_urls.sub));
        self.signal_sources_changed_waker.wake();

        select! {
            events_stream_manager_runner_error = events_stream_manager_runner => events_stream_manager_runner_error,
            _ = events_stream_manager_receiver_runner => panic!("events_stream_manager_receiver_runner yielded"),
            snapshot_runner_runner_error = snapshot_runner_runner => snapshot_runner_runner_error,
        }
    }

    const ERROR_RESTART_INTERVAL: Duration = Duration::from_secs(10);
    async fn run(&self) -> ! {
        loop {
            let error = self.run_once().await.context("run_once");
            self.failed();

            log::error!("device {} failed: {:?}", self.configuration.host, error);
            tokio::time::sleep(Self::ERROR_RESTART_INTERVAL).await;
        }
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("hikvision/ds2cd2x32x_x")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
    fn as_gui_summary_provider(&self) -> Option<&dyn GuiSummaryProvider> {
        Some(self)
    }
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
        Some(self)
    }
}
#[async_trait]
impl Runnable for Device {
    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let run_future = self.run();
        pin_mut!(run_future);
        let mut run_future = run_future.fuse();

        select! {
            _ = run_future => panic!("run_future yielded"),
            () = exit_flag => {},
        }

        Exited
    }
}
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        // Will never be called - no targets
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_rtsp_url_main as &dyn signal::Base,
            1 => &self.signal_rtsp_url_sub as &dyn signal::Base,

            100 => &self.signal_event_camera_failure as &dyn signal::Base,
            101 => &self.signal_event_video_loss as &dyn signal::Base,
            102 => &self.signal_event_tampering_detection as &dyn signal::Base,
            103 => &self.signal_event_motion_detection as &dyn signal::Base,
            104 => &self.signal_event_line_detection as &dyn signal::Base,
            105 => &self.signal_event_field_detection as &dyn signal::Base,
        }
    }
}
impl GuiSummaryProvider for Device {
    fn value(&self) -> Box<dyn devices::GuiSummary> {
        Box::new(self.device_state.read().clone())
    }
    fn waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}
impl uri_cursor::Handler for Device {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next("snapshot", uri_cursor) => {
                self.snapshot_manager.handle(request, uri_cursor)
            }
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
