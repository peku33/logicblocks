use super::{
    device::{Device, DeviceContext},
    signals_runner::{
        Connections as ConnectionsGeneric, DeviceIdSignalId as DeviceIdSignalIdGeneric,
        SignalsRunner as SignalsRunnerGeneric,
    },
};
use crate::{
    util::{
        atomic_cell::AtomicCell,
        select_all_empty::{JoinAllEmptyUnit, SelectAllEmptyFutureInfinite},
        tokio_cancelable::ScopedSpawn,
    },
    web::{
        uri_cursor::{Handler, UriCursor},
        Request, Response,
    },
};
use futures::{
    future::{BoxFuture, FutureExt},
    pin_mut, select,
};
use http::Method;
use owning_ref::OwningHandle;
use std::collections::HashMap;

pub type DeviceId = u32;

type DeviceContextRunContext<'d> =
    OwningHandle<Box<DeviceContext<'d>>, Box<AtomicCell<ScopedSpawn<'d, !>>>>;
fn device_context_run_context_build(device_context: DeviceContext) -> DeviceContextRunContext {
    DeviceContextRunContext::new_with_fn(Box::new(device_context), unsafe {
        |device_context_ptr| {
            Box::new(AtomicCell::new(ScopedSpawn::new(
                (*device_context_ptr).run().boxed(),
            )))
        }
    })
}

pub type SignalsRunner = SignalsRunnerGeneric<DeviceId>;
pub type Connections = ConnectionsGeneric<DeviceId>;
pub type DeviceIdSignalId = DeviceIdSignalIdGeneric<DeviceId>;

type SignalsRunnerRunContext =
    OwningHandle<Box<SignalsRunner>, Box<AtomicCell<BoxFuture<'static, !>>>>;
fn signals_runner_run_context_build(signals_runner: SignalsRunner) -> SignalsRunnerRunContext {
    SignalsRunnerRunContext::new_with_fn(Box::new(signals_runner), unsafe {
        |signals_runner_ptr| Box::new(AtomicCell::new((*signals_runner_ptr).run().boxed()))
    })
}

pub struct Runner<'d> {
    device_context_run_contexts: HashMap<DeviceId, DeviceContextRunContext<'d>>,
    signals_runner_run_context: SignalsRunnerRunContext,
}
impl<'d> Runner<'d> {
    pub fn new(
        devices: HashMap<DeviceId, Box<dyn Device + 'd>>,
        connections: Connections,
    ) -> Self {
        log::trace!("new called");

        let signals_runner = Self::build_signals_runner(&devices, &connections);
        let signals_runner_run_context = signals_runner_run_context_build(signals_runner);

        let device_context_run_contexts = devices
            .into_iter()
            .map(|(device_id, device)| {
                (
                    device_id,
                    device_context_run_context_build(DeviceContext::new(device)),
                )
            })
            .collect();

        Self {
            device_context_run_contexts,
            signals_runner_run_context,
        }
    }

    fn build_signals_runner(
        devices: &HashMap<DeviceId, Box<dyn Device + 'd>>,
        connections: &Connections,
    ) -> SignalsRunner {
        SignalsRunner::new(
            devices
                .iter()
                .flat_map(move |(device_id, device)| {
                    device
                        .signals()
                        .into_iter()
                        .map(move |(signal_id, signal)| {
                            (
                                DeviceIdSignalId::new(*device_id, signal_id),
                                signal.remote(),
                            )
                        })
                })
                .collect(),
            connections,
        )
    }

    pub async fn run(&self) -> ! {
        log::trace!("run called");

        let device_context_run_contexts_run =
            Self::run_device_context_run_contexts(&self.device_context_run_contexts);
        pin_mut!(device_context_run_contexts_run);
        let mut device_context_run_contexts_run = device_context_run_contexts_run.fuse();

        let signals_runner_run_context_run =
            Self::run_signals_runner_run_context(&self.signals_runner_run_context);
        pin_mut!(signals_runner_run_context_run);
        let mut signals_runner_run_context_run = signals_runner_run_context_run.fuse();

        select! {
            _ = device_context_run_contexts_run => panic!("device_context_run_contexts_run yielded"),
            _ = signals_runner_run_context_run => panic!("signals_runner_run_context_run yielded"),
        }
    }
    async fn run_device_context_run_contexts(
        device_context_run_contexts: &HashMap<DeviceId, DeviceContextRunContext<'d>>
    ) -> ! {
        device_context_run_contexts
            .values()
            .map(move |device_context_run_context| {
                Self::run_device_context_run_context(device_context_run_context)
            })
            .collect::<SelectAllEmptyFutureInfinite<_>>()
            .await
    }
    async fn run_device_context_run_context(
        device_context_run_context: &DeviceContextRunContext<'d>
    ) -> ! {
        let mut scoped_spawn = device_context_run_context.lease();
        (&mut *scoped_spawn).await.unwrap()
    }
    async fn run_signals_runner_run_context(
        signals_runner_run_context: &SignalsRunnerRunContext
    ) -> ! {
        let mut run = signals_runner_run_context.lease();
        (&mut *run).await
    }

    pub async fn finalize(self) {
        log::trace!("finalize begin");

        Self::finalize_device_context_run_contexts(self.device_context_run_contexts).await;

        log::trace!("finalize end");
    }
    async fn finalize_device_context_run_contexts(
        device_context_run_contexts: HashMap<DeviceId, DeviceContextRunContext<'d>>
    ) {
        log::trace!("finalize_device_context_run_contexts begin");

        device_context_run_contexts
            .into_iter()
            .map(|(_, device_context_run_context)| {
                Self::finalize_device_context_run_context(device_context_run_context)
            })
            .collect::<JoinAllEmptyUnit<_>>()
            .await;

        log::trace!("finalize_device_context_run_contexts end");
    }
    async fn finalize_device_context_run_context(
        device_context_run_context: DeviceContextRunContext<'_>
    ) {
        log::trace!("finalize_device_context_run_context begin");

        device_context_run_context.lease().finalize().await;
        device_context_run_context
            .into_owner()
            .into_device()
            .finalize()
            .await;

        log::trace!("finalize_device_context_run_context end");
    }
}
impl<'p> Handler for Runner<'p> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        match (request.method(), uri_cursor.next_item()) {
            (&Method::GET, ("devices", None)) => {
                let device_ids = self
                    .device_context_run_contexts
                    .keys()
                    .copied()
                    .collect::<Vec<_>>();
                async move { Response::ok_json(device_ids) }.boxed()
            }
            (_, (device_id_str, Some(uri_cursor_next))) => {
                let device_id: DeviceId = match device_id_str.parse() {
                    Ok(device_id) => device_id,
                    Err(error) => {
                        return async move { Response::error_400_from_error(error) }.boxed()
                    }
                };
                let device_context_run_context =
                    match self.device_context_run_contexts.get(&device_id) {
                        Some(device_context_run_context) => device_context_run_context,
                        None => return async move { Response::error_404() }.boxed(),
                    };
                device_context_run_context
                    .as_owner()
                    .handle(request, uri_cursor_next)
            }
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
