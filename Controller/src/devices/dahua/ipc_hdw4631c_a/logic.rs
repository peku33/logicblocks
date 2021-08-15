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
        runtime::{Exited, Runnable},
        waker_stream,
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
    sub1: IpcRtspUrl,
    sub2: IpcRtspUrl,
}

#[derive(Serialize, Copy, Clone, Default, Debug)]
pub struct Events {
    video_blind: bool,
    scene_change: bool,
    video_motion: bool,
    audio_mutation: bool,
}
impl Events {
    pub fn from_event_stream_events(hardware_events: &event_stream::Events) -> Self {
        let mut video_blind: bool = false;
        let mut scene_change: bool = false;
        let mut video_motion: bool = false;
        let mut audio_mutation: bool = false;

        hardware_events.iter().for_each(|event| match event {
            event_stream::Event::VideoBlind => {
                video_blind = true;
            }
            event_stream::Event::SceneChange => {
                scene_change = true;
            }
            event_stream::Event::VideoMotion { region: _ } => {
                video_motion = true;
            }
            event_stream::Event::AudioMutation => {
                audio_mutation = true;
            }
        });

        Self {
            video_blind,
            scene_change,
            video_motion,
            audio_mutation,
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
    signal_rtsp_url_sub1: signal::state_source::Signal<IpcRtspUrl>,
    signal_rtsp_url_sub2: signal::state_source::Signal<IpcRtspUrl>,
    signal_event_video_blind: signal::state_source::Signal<bool>,
    signal_event_scene_change: signal::state_source::Signal<bool>,
    signal_event_video_motion: signal::state_source::Signal<bool>,
    signal_event_audio_mutation: signal::state_source::Signal<bool>,
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
            signal_rtsp_url_sub1: signal::state_source::Signal::<IpcRtspUrl>::new(None),
            signal_rtsp_url_sub2: signal::state_source::Signal::<IpcRtspUrl>::new(None),
            signal_event_video_blind: signal::state_source::Signal::<bool>::new(None),
            signal_event_scene_change: signal::state_source::Signal::<bool>::new(None),
            signal_event_video_motion: signal::state_source::Signal::<bool>::new(None),
            signal_event_audio_mutation: signal::state_source::Signal::<bool>::new(None),
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
            .signal_event_video_blind
            .set_one(Some(events.video_blind));
        signals_changed |= self
            .signal_event_scene_change
            .set_one(Some(events.scene_change));
        signals_changed |= self
            .signal_event_video_motion
            .set_one(Some(events.video_motion));
        signals_changed |= self
            .signal_event_audio_mutation
            .set_one(Some(events.audio_mutation));
        if signals_changed {
            self.signal_sources_changed_waker.wake();
        }
    }

    fn failed(&self) {
        *self.device_state.write() = DeviceState::Error;
        self.gui_summary_waker.wake();

        self.snapshot_manager.image_unset();

        let _ = self.signal_rtsp_url_main.set_one(None);
        let _ = self.signal_rtsp_url_sub1.set_one(None);
        let _ = self.signal_rtsp_url_sub2.set_one(None);
        let _ = self.signal_event_video_blind.set_one(None);
        let _ = self.signal_event_scene_change.set_one(None);
        let _ = self.signal_event_video_motion.set_one(None);
        let _ = self.signal_event_audio_mutation.set_one(None);
        self.signal_sources_changed_waker.wake();
    }

    async fn run_once(&self) -> Result<!, Error> {
        *self.device_state.write() = DeviceState::Initializing;
        self.gui_summary_waker.wake();

        // api
        let api = api::Api::new(
            self.configuration.host.clone(),
            self.configuration.admin_password.clone(),
        );

        // basic validation
        let basic_device_info = api
            .validate_basic_device_info()
            .await
            .context("validate_basic_device_info")?;

        // configuration & watcher credentials
        let (shared_user_login, shared_user_password) =
            match &self.configuration.hardware_configuration {
                HardwareConfiguration::Full {
                    hardware_configuration,
                } => {
                    let mut configurator =
                        configurator::Configurator::new(&api, &basic_device_info);
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

        let rtsp_urls = RtspUrls {
            main: IpcRtspUrl(api.rtsp_url_build(
                shared_user_login,
                shared_user_password,
                api::VideoStream::Main,
            )),
            sub1: IpcRtspUrl(api.rtsp_url_build(
                shared_user_login,
                shared_user_password,
                api::VideoStream::Sub1,
            )),
            sub2: IpcRtspUrl(api.rtsp_url_build(
                shared_user_login,
                shared_user_password,
                api::VideoStream::Sub2,
            )),
        };

        // event manager
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

        // snapshot runner
        let snapshot_runner = SnapshotRunner::new(
            &self.snapshot_manager,
            || api.snapshot_retry(2),
            || self.snapshot_updated_handle(),
            SNAPSHOT_INTERVAL,
        );
        let snapshot_runner_runner = snapshot_runner.run_once();
        pin_mut!(snapshot_runner_runner);
        let mut snapshot_runner_runner = snapshot_runner_runner.fuse();

        // device is ready
        *self.device_state.write() = DeviceState::Running {
            snapshot_updated: None,
            rtsp_urls: rtsp_urls.clone(),
            events: Events::default(),
        };
        self.gui_summary_waker.wake();

        // signal values
        let _ = self.signal_rtsp_url_main.set_one(Some(rtsp_urls.main));
        let _ = self.signal_rtsp_url_sub1.set_one(Some(rtsp_urls.sub1));
        let _ = self.signal_rtsp_url_sub2.set_one(Some(rtsp_urls.sub2));
        self.signal_sources_changed_waker.wake();

        // run
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
        Cow::from("dahua/ipc_hdw4631c_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
    fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
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
        // will never be called - no targets
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_rtsp_url_main as &dyn signal::Base,
            1 => &self.signal_rtsp_url_sub1 as &dyn signal::Base,
            1 => &self.signal_rtsp_url_sub2 as &dyn signal::Base,

            100 => &self.signal_event_video_blind as &dyn signal::Base,
            101 => &self.signal_event_scene_change as &dyn signal::Base,
            102 => &self.signal_event_video_motion as &dyn signal::Base,
            103 => &self.signal_event_audio_mutation as &dyn signal::Base,
        }
    }
}
impl devices::GuiSummaryProvider for Device {
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
