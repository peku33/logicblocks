use super::{
    device::{Device, DeviceContext},
    signal::SignalRemoteBase,
    signals_runner::DeviceIdSignalId,
};
use crate::{
    util::{select_all_empty::SelectAllEmptyFutureInfinite, tokio_cancelable::ScopedSpawn},
    web::{
        uri_cursor::{Handler, UriCursor},
        Request, Response,
    },
};
use futures::{
    channel::mpsc,
    future::{BoxFuture, FutureExt, JoinAll},
    select,
    stream::{BoxStream, StreamExt},
};
use owning_ref::OwningHandle;
use std::{
    collections::{hash_map, HashMap, HashSet},
    fmt,
    mem::replace,
    sync::Mutex,
};
use tokio::sync::RwLock;

pub type DeviceId = u32;

pub trait DeviceProvider: Sync + fmt::Debug {
    fn get_change_stream(&self) -> BoxStream<()>;
    fn get_device_ids(&self) -> HashSet<DeviceId>;
    fn get_device(
        &self,
        device_id: DeviceId,
    ) -> Option<Box<dyn Device>>;
}

type DeviceContextRunner = OwningHandle<Box<DeviceContext>, Box<Mutex<ScopedSpawn<'static, !>>>>;
fn device_context_runner_build(device_context: DeviceContext) -> DeviceContextRunner {
    DeviceContextRunner::new_with_fn(Box::new(device_context), unsafe {
        |device_context_ptr| {
            Box::new(Mutex::new(ScopedSpawn::new(
                (*device_context_ptr).run().boxed(),
            )))
        }
    })
}

pub struct DeviceProviderContext<'p> {
    device_provider: &'p dyn DeviceProvider,
    device_context_runners: RwLock<HashMap<DeviceId, DeviceContextRunner>>,
    device_list_changed_sender: mpsc::UnboundedSender<()>,
}
impl<'p> DeviceProviderContext<'p> {
    pub fn new(
        device_provider: &'p dyn DeviceProvider,
        device_list_changed_sender: mpsc::UnboundedSender<()>,
    ) -> Self {
        log::trace!("new called");

        let device_context_runners = RwLock::new(HashMap::new());

        Self {
            device_provider,
            device_context_runners,
            device_list_changed_sender,
        }
    }

    pub async fn get_device_ids(&self) -> HashSet<DeviceId> {
        let device_context_runners = self.device_context_runners.read().await;

        device_context_runners.keys().copied().collect()
    }

    pub async fn get_signals_remote_bases(
        &self
    ) -> HashMap<DeviceIdSignalId<DeviceId>, SignalRemoteBase> {
        let device_context_runners = self.device_context_runners.read().await;

        device_context_runners
            .iter()
            .flat_map(move |(device_id, device_context_runner)| {
                let device_id = *device_id;
                device_context_runner
                    .as_owner()
                    .get_device()
                    .get_signals()
                    .into_iter()
                    .map(move |(signal_id, signal_base)| {
                        (
                            DeviceIdSignalId::new(device_id, signal_id),
                            signal_base.remote(),
                        )
                    })
            })
            .collect()
    }

    // We don't use handler here, as we need to explicitly pass device_id which is unpacked by parent
    pub async fn web_handle(
        &self,
        device_id: DeviceId,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        let device_context_runners = self.device_context_runners.read().await;
        let device_context = match device_context_runners.get(&device_id) {
            Some(device_context_runner) => device_context_runner.as_owner(),
            None => return async move { Response::error_404() }.boxed(),
        };
        device_context.handle(request, uri_cursor)
    }

    async fn reload_device_context_runners(&self) {
        log::trace!("reload_device_context_runners called");

        let device_ids = self.device_provider.get_device_ids();

        if device_ids
            == self
                .device_context_runners
                .read()
                .await
                .keys()
                .copied()
                .collect()
        {
            log::trace!("no changes detected");

            return;
        }

        let mut device_context_runners = self.device_context_runners.write().await;

        // Iterate current devices
        let device_context_runners_old = replace(&mut *device_context_runners, HashMap::new());
        for (device_id, device_context_runner) in device_context_runners_old.into_iter() {
            if device_ids.contains(&device_id) {
                // Retain device
                log::trace!("retaining device context: {:?}", device_id);

                assert!(device_context_runners
                    .insert(device_id, device_context_runner)
                    .is_none());
            } else {
                // Finalize device
                log::trace!("finalizing device context: {:?}", device_id);

                Self::finalize_device_context_runner(device_context_runner).await;
            }
        }

        // Iterate new devices
        for device_id in device_ids {
            match device_context_runners.entry(device_id) {
                hash_map::Entry::Occupied(_) => {
                    // Already exists
                }
                hash_map::Entry::Vacant(entry) => {
                    match self.device_provider.get_device(device_id) {
                        None => {
                            log::warn!(
                                "device {:?} reported by get_device_ids is missing",
                                device_id
                            );
                        }
                        Some(device) => {
                            log::trace!("spawning device context: {:?}", device_id);

                            let device_context = DeviceContext::new(device);
                            let device_context_runner = device_context_runner_build(device_context);
                            entry.insert(device_context_runner);
                        }
                    }
                }
            }
        }

        // Notify about changes
        self.device_list_changed_sender.unbounded_send(()).unwrap();
    }

    pub async fn run(&self) -> ! {
        log::trace!("run called");

        let device_provider_change_stream = self.device_provider.get_change_stream();
        let mut device_provider_change_stream = device_provider_change_stream.fuse();

        self.reload_device_context_runners().await;

        log::trace!("run entering main loop");

        loop {
            select! {
                () = device_provider_change_stream.select_next_some() => {
                    log::trace!("device_provider_change_stream yielded, calling reload_device_context_runners()");
                    self.reload_device_context_runners().await;
                },
                _ = self.run_device_context_runners().fuse() => {
                    panic!("run_device_context_runner_future yielded");
                },
            }
        }
    }
    async fn run_device_context_runners(&self) -> ! {
        log::trace!("run_device_context_runners called");

        let device_context_runners = self.device_context_runners.read().await;

        device_context_runners
            .values()
            .map(|device_context_runner| Self::run_device_context_runner(device_context_runner))
            .collect::<SelectAllEmptyFutureInfinite<_>>()
            .await
    }
    async fn run_device_context_runner(device_context_runner: &DeviceContextRunner) -> ! {
        log::trace!("run_device_context_runner called");

        let mut run_scoped_spawn = device_context_runner.try_lock().unwrap();
        let run_scoped_spawn = &mut *run_scoped_spawn;
        run_scoped_spawn.await.unwrap()
    }

    pub async fn finalize(self) {
        log::trace!("finalize begin");

        let device_context_runners = self.device_context_runners.into_inner();
        Self::finalize_device_context_runners(device_context_runners).await;

        log::trace!("finalize end");
    }
    async fn finalize_device_context_runners(
        device_context_runners: HashMap<DeviceId, DeviceContextRunner>
    ) {
        log::trace!("finalize_device_context_runners begin");

        device_context_runners
            .into_iter()
            .map(|(_, device_context_runner)| {
                Self::finalize_device_context_runner(device_context_runner)
            })
            .collect::<JoinAll<_>>()
            .await;

        log::trace!("finalize_device_context_runners end");
    }
    async fn finalize_device_context_runner(device_context_runner: DeviceContextRunner) {
        log::trace!("finalize_device_context_runner begin");

        {
            let mut run_scoped_spawn = device_context_runner.try_lock().unwrap();
            let run_scoped_spawn = &mut *run_scoped_spawn;
            run_scoped_spawn.finalize().await;
        };

        let device_context = device_context_runner.into_owner();
        device_context.finalize().await;

        log::trace!("finalize_device_context_runner end");
    }
}
