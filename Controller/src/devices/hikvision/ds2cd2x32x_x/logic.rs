use super::hardware::{api, configurator, event_stream};
use crate::{
    datatypes::ipc_rtsp_url::IpcRtspUrl,
    devices::{
        self,
        soft::surveillance::snapshot::logic_device_inner::{
            Manager as SnapshotManager, Runner as SnapshotRunner,
        },
    },
    signals::{self, signal},
    util::{
        async_flag,
        runnable::{Exited, Runnable},
    },
    web::{self, uri_cursor},
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

#[derive(Debug)]
pub enum ConfigurationHardware {
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
    pub hardware: ConfigurationHardware,
}

#[derive(Clone, Debug, Serialize)]
pub struct RtspUrls {
    main: IpcRtspUrl,
    sub: IpcRtspUrl,
}

#[derive(Clone, Copy, Default, Debug, Serialize)]
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

#[derive(Clone, Debug, Serialize)]
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

    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_rtsp_url_main: signal::state_source::Signal<IpcRtspUrl>,
    signal_rtsp_url_sub: signal::state_source::Signal<IpcRtspUrl>,
    signal_event_camera_failure: signal::state_source::Signal<bool>,
    signal_event_video_loss: signal::state_source::Signal<bool>,
    signal_event_tampering_detection: signal::state_source::Signal<bool>,
    signal_event_motion_detection: signal::state_source::Signal<bool>,
    signal_event_line_detection: signal::state_source::Signal<bool>,
    signal_event_field_detection: signal::state_source::Signal<bool>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            device_state: RwLock::new(DeviceState::Initializing),
            snapshot_manager: SnapshotManager::new(),

            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_rtsp_url_main: signal::state_source::Signal::<IpcRtspUrl>::new(None),
            signal_rtsp_url_sub: signal::state_source::Signal::<IpcRtspUrl>::new(None),
            signal_event_camera_failure: signal::state_source::Signal::<bool>::new(None),
            signal_event_video_loss: signal::state_source::Signal::<bool>::new(None),
            signal_event_tampering_detection: signal::state_source::Signal::<bool>::new(None),
            signal_event_motion_detection: signal::state_source::Signal::<bool>::new(None),
            signal_event_line_detection: signal::state_source::Signal::<bool>::new(None),
            signal_event_field_detection: signal::state_source::Signal::<bool>::new(None),

            gui_summary_waker: devices::gui_summary::Waker::new(),
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

        let mut signals_sources_changed = false;
        signals_sources_changed |= self
            .signal_event_camera_failure
            .set_one(Some(events.camera_failure));

        signals_sources_changed |= self
            .signal_event_video_loss
            .set_one(Some(events.video_loss));
        signals_sources_changed |= self
            .signal_event_tampering_detection
            .set_one(Some(events.tampering_detection));
        signals_sources_changed |= self
            .signal_event_motion_detection
            .set_one(Some(events.motion_detection));
        signals_sources_changed |= self
            .signal_event_line_detection
            .set_one(Some(events.line_detection));
        signals_sources_changed |= self
            .signal_event_field_detection
            .set_one(Some(events.field_detection));
        if signals_sources_changed {
            self.signals_sources_changed_waker.wake();
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
        self.signals_sources_changed_waker.wake();
    }

    pub const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(60);
    async fn run_once(&self) -> Result<!, Error> {
        *self.device_state.write() = DeviceState::Initializing;
        self.gui_summary_waker.wake();

        // Build client
        let api = api::Api::new(
            self.configuration.host.clone(),
            self.configuration.admin_password.clone(),
        );

        // Set device configuration
        // Get rtsp data based on configuration type
        let (shared_user_login, shared_user_password) = match &self.configuration.hardware {
            ConfigurationHardware::Full {
                hardware_configuration,
            } => {
                let mut configurator = configurator::Configurator::connect(&api)
                    .await
                    .context("connect")?;
                configurator
                    .configure(hardware_configuration.clone())
                    .await
                    .context("configure")?;

                (
                    configurator::Configurator::SHARED_USER_LOGIN,
                    &hardware_configuration.shared_user_password,
                )
            }
            ConfigurationHardware::Skip {
                shared_user_login,
                shared_user_password,
            } => {
                // Check device
                let _basic_device_info = api
                    .validate_basic_device_info()
                    .await
                    .context("validate_basic_device_info")?;

                (shared_user_login.as_str(), shared_user_password)
            }
        };

        // Set device video URLs
        let rtsp_urls = RtspUrls {
            main: IpcRtspUrl(api.rtsp_url_build(
                shared_user_login,
                shared_user_password,
                api::VideoStream::Main,
            )),
            sub: IpcRtspUrl(api.rtsp_url_build(
                shared_user_login,
                shared_user_password,
                api::VideoStream::Sub,
            )),
        };

        // Attach event manager
        let events_stream_manager = event_stream::Manager::new(&api);

        let events_stream_manager_receiver_runner =
            tokio_stream::wrappers::WatchStream::new(events_stream_manager.receiver())
                .for_each(async |hardware_events| {
                    let events = Events::from_event_stream_events(&hardware_events);
                    self.events_handle(events);
                })
                .fuse();
        pin_mut!(events_stream_manager_receiver_runner);

        let events_stream_manager_runner = events_stream_manager.run_once().fuse();
        pin_mut!(events_stream_manager_runner);

        // Attach snapshot manager
        let snapshot_runner = SnapshotRunner::new(
            &self.snapshot_manager,
            || api.snapshot(),
            || self.snapshot_updated_handle(),
            Self::SNAPSHOT_INTERVAL,
        );
        let snapshot_runner_runner = snapshot_runner.run_once().fuse();
        pin_mut!(snapshot_runner_runner);

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
        self.signals_sources_changed_waker.wake();

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
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
    fn as_gui_summary_device_base(&self) -> Option<&dyn devices::gui_summary::DeviceBase> {
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
        let runner = self.run().fuse();
        pin_mut!(runner);

        select! {
            _ = runner => panic!("runner yielded"),
            () = exit_flag => {},
        }

        Exited
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalIdentifier {
    RtspUrlMain,
    RtspUrlSub,

    EventCameraFailure,
    EventVideoLoss,
    EventTamperingDetection,
    EventMotionDetection,
    EventLineDetection,
    EventFieldDetection,
}
impl signals::Identifier for SignalIdentifier {}
impl signals::Device for Device {
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        None
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        Some(&self.signals_sources_changed_waker)
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::RtspUrlMain => &self.signal_rtsp_url_main as &dyn signal::Base,
            SignalIdentifier::RtspUrlSub => &self.signal_rtsp_url_sub as &dyn signal::Base,
            SignalIdentifier::EventCameraFailure => &self.signal_event_camera_failure as &dyn signal::Base,
            SignalIdentifier::EventVideoLoss => &self.signal_event_video_loss as &dyn signal::Base,
            SignalIdentifier::EventTamperingDetection => &self.signal_event_tampering_detection as &dyn signal::Base,
            SignalIdentifier::EventMotionDetection => &self.signal_event_motion_detection as &dyn signal::Base,
            SignalIdentifier::EventLineDetection => &self.signal_event_line_detection as &dyn signal::Base,
            SignalIdentifier::EventFieldDetection => &self.signal_event_field_detection as &dyn signal::Base,
        }
    }
}

impl devices::gui_summary::Device for Device {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = DeviceState;
    fn value(&self) -> Self::Value {
        self.device_state.read().clone()
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
            _ => async { web::Response::error_404() }.boxed(),
        }
    }
}
