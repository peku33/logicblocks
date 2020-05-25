use super::{
    device_provider::DeviceProvider,
    device_providers::{DevicePoolProvidersContext, DeviceProviderId, DeviceProviderIdDeviceId},
    signals_runner::{
        Connections as ConnectionsGeneric, DeviceIdSignalId as DeviceIdSignalIdGeneric,
        SignalsRunner as SignalsRunnerGeneric,
    },
};
use crate::web::{
    uri_cursor::{Handler, UriCursor},
    Request, Response,
};
use futures::{
    channel::mpsc,
    future::{BoxFuture, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use owning_ref::OwningHandle;
use std::{
    collections::HashMap,
    mem::{forget, replace, MaybeUninit},
    sync::Mutex,
};
use tokio::sync::RwLock;

pub type SignalsRunner = SignalsRunnerGeneric<DeviceProviderIdDeviceId>;
pub type Connections = ConnectionsGeneric<DeviceProviderIdDeviceId>;
pub type DeviceIdSignalId = DeviceIdSignalIdGeneric<DeviceProviderIdDeviceId>;

type SignalsRunnerRunContext = OwningHandle<Box<SignalsRunner>, Box<Mutex<BoxFuture<'static, !>>>>;
fn signals_runner_run_context_build(signals_runner: SignalsRunner) -> SignalsRunnerRunContext {
    SignalsRunnerRunContext::new_with_fn(Box::new(signals_runner), unsafe {
        |signals_runner_ptr| Box::new(Mutex::new((*signals_runner_ptr).run().boxed()))
    })
}

pub struct Runner<'p> {
    device_pool_provider_context: DevicePoolProvidersContext<'p>,
    device_list_changed_sender: mpsc::UnboundedSender<()>,
    device_list_changed_receiver: Mutex<mpsc::UnboundedReceiver<()>>,

    signals_runner_run_context: RwLock<SignalsRunnerRunContext>,

    connections: RwLock<Connections>,
    connections_sender: mpsc::UnboundedSender<Connections>,
    connections_receiver: Mutex<mpsc::UnboundedReceiver<Connections>>,
}
impl<'p> Runner<'p> {
    pub fn new(
        device_providers: HashMap<DeviceProviderId, &'p dyn DeviceProvider>,
        connections: Connections,
    ) -> Self {
        log::trace!("new called");

        let (device_list_changed_sender, device_list_changed_receiver) = mpsc::unbounded();
        let device_list_changed_receiver = Mutex::new(device_list_changed_receiver);

        let device_pool_provider_context =
            DevicePoolProvidersContext::new(device_providers, &device_list_changed_sender);

        // Stub value, will be immediately reloaded during run
        let signals_runner_run_context = RwLock::new(signals_runner_run_context_build(
            SignalsRunner::new(HashMap::new(), &Connections::new()),
        ));

        let connections = RwLock::new(connections);
        let (connections_sender, connections_receiver) = mpsc::unbounded();
        let connections_receiver = Mutex::new(connections_receiver);

        Self {
            device_pool_provider_context,
            device_list_changed_sender,
            device_list_changed_receiver,

            signals_runner_run_context,

            connections,
            connections_sender,
            connections_receiver,
        }
    }

    async fn rebuild_signals_runner(&self) {
        log::trace!("rebuild_signals_runner called");

        let mut signals_runner_run_context = self.signals_runner_run_context.write().await;

        // Build stub
        let signals_runner_run_context_stub = MaybeUninit::<SignalsRunnerRunContext>::uninit();

        // Replace old with stub
        let signals_runner_run_context_old = replace(&mut *signals_runner_run_context, unsafe {
            signals_runner_run_context_stub.assume_init()
        });

        // Drop old
        drop(signals_runner_run_context_old);

        // Build new one
        let signals_runner_run_context_new = signals_runner_run_context_build({
            let signals_remote_bases = self
                .device_pool_provider_context
                .get_signals_remote_bases()
                .await;

            let connections = self.connections.read().await;

            SignalsRunner::new(signals_remote_bases, &connections)
        });

        // Replace stub with new one
        let signals_runner_run_context_stub = replace(
            &mut *signals_runner_run_context,
            signals_runner_run_context_new,
        );

        // Don't drop stub
        forget(signals_runner_run_context_stub);
    }

    pub fn connections_set(
        &self,
        connections: Connections,
    ) {
        log::trace!("connections_set called");
        self.connections_sender.unbounded_send(connections).unwrap();
    }

    pub async fn run(&self) -> ! {
        log::trace!("run called");

        let device_pool_provider_context_run = self.device_pool_provider_context.run();
        pin_mut!(device_pool_provider_context_run);
        let mut device_pool_provider_context_run = device_pool_provider_context_run.fuse();

        let mut device_list_changed_receiver =
            self.device_list_changed_receiver.try_lock().unwrap();
        let mut device_list_changed_receiver = device_list_changed_receiver.by_ref().fuse();

        let mut connections_receiver = self.connections_receiver.try_lock().unwrap();
        let mut connections_receiver = connections_receiver.by_ref().fuse();

        // This will be called by device rebuild notifications
        // self.rebuild_signals_runner().await;

        log::trace!("run entering main loop");

        loop {
            select! {
                _ = device_pool_provider_context_run => {
                    panic!("device_pool_provider_context_run yielded");
                },
                _ = self.run_signals_runner_run_context().fuse() => {
                    panic!("run_signals_runner_run_context yielded");
                },
                () = device_list_changed_receiver.select_next_some() => {
                    log::trace!("device_list_changed_receiver yielded, calling rebuild_signals_runner");

                    self.rebuild_signals_runner().await;
                },
                connections = connections_receiver.select_next_some() => {
                    log::trace!("connections_receiver yielded, calling rebuild_signals_runner");

                    *self.connections.write().await = connections;
                    self.rebuild_signals_runner().await;
                },
            }
        }
    }
    async fn run_signals_runner_run_context(&self) -> ! {
        log::trace!("run_signals_runner_run_context called");

        let signals_runner_run_context = self.signals_runner_run_context.read().await;
        let mut signals_runner_run_context_run = signals_runner_run_context.try_lock().unwrap();
        let signals_runner_run_context_run = &mut *signals_runner_run_context_run;
        signals_runner_run_context_run.await
    }

    pub async fn finalize(self) {
        log::trace!("finalize begin");

        self.device_pool_provider_context.finalize().await;

        log::trace!("finalize end");
    }
}
impl<'p> Handler for Runner<'p> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor.next_item() {
            ("devices", Some(uri_cursor_next_item)) => self
                .device_pool_provider_context
                .handle(request, uri_cursor_next_item),
            // ("signals", Some(uri_cursor_next_item)) => {
            //     todo!();
            // }
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
