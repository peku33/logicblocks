use super::{
    device_provider::{DeviceId, DeviceProvider, DeviceProviderContext},
    signal::SignalRemoteBase,
    signals_runner::DeviceIdSignalId,
};
use crate::{
    util::select_all_empty::SelectAllEmptyFutureInfinite,
    web::{
        uri_cursor::{handler_async_bridge::HandlerAsyncBridge, Handler, HandlerAsync, UriCursor},
        Request, Response,
    },
};
use failure::Error;
use futures::{
    channel::mpsc,
    future::{BoxFuture, FutureExt, JoinAll},
    pin_mut, select,
};
use http::Method;
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
};

pub type DeviceProviderId = u16;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct DeviceProviderIdDeviceId {
    device_provider_id: DeviceProviderId,
    device_id: DeviceId,
}
impl DeviceProviderIdDeviceId {
    pub fn new(
        device_provider_id: DeviceProviderId,
        device_id: DeviceId,
    ) -> Self {
        Self {
            device_provider_id,
            device_id,
        }
    }
    pub fn from_combined(combined: u64) -> Result<Self, Error> {
        let device_provider_id = combined & (((1 << 32) - 1) << 32);
        let device_id = combined & ((1 << 32) - 1);

        let device_provider_id: DeviceProviderId = device_provider_id.try_into()?;
        let device_id: DeviceId = device_id.try_into()?;

        Ok(Self {
            device_provider_id,
            device_id,
        })
    }
    pub fn to_combined(self) -> u64 {
        ((self.device_provider_id as u64) << 32) | (self.device_id as u64)
    }

    pub fn device_provider_id(self) -> DeviceProviderId {
        self.device_provider_id
    }
    pub fn device_id(self) -> DeviceId {
        self.device_id
    }
}

pub struct DevicePoolProvidersContext<'p> {
    device_provider_contexts: HashMap<DeviceProviderId, DeviceProviderContext<'p>>,
    handler_async_bridge: HandlerAsyncBridge,
}
impl<'p> DevicePoolProvidersContext<'p> {
    pub fn new(
        device_providers: HashMap<DeviceProviderId, &'p dyn DeviceProvider>,
        device_list_changed_sender: &mpsc::UnboundedSender<()>,
    ) -> Self {
        log::trace!("new called");

        let device_provider_contexts = device_providers
            .into_iter()
            .map(|(device_id, device_provider)| {
                (
                    device_id,
                    DeviceProviderContext::new(device_provider, device_list_changed_sender.clone()),
                )
            })
            .collect::<HashMap<_, _>>();

        let handler_async_bridge = HandlerAsyncBridge::new();

        Self {
            device_provider_contexts,
            handler_async_bridge,
        }
    }

    pub async fn get_device_provider_id_device_ids(&self) -> HashSet<DeviceProviderIdDeviceId> {
        self.device_provider_contexts
            .iter()
            .map(move |(device_provider_id, device_provider_context)| {
                let device_provider_id = *device_provider_id;
                device_provider_context
                    .get_device_ids()
                    .map(move |device_ids| (device_provider_id, device_ids))
            })
            .collect::<JoinAll<_>>()
            .await
            .into_iter()
            .flat_map(move |(device_provider_id, device_ids)| {
                device_ids.into_iter().map(move |device_id| {
                    DeviceProviderIdDeviceId::new(device_provider_id, device_id)
                })
            })
            .collect()
    }
    pub async fn get_signals_remote_bases(
        &self
    ) -> HashMap<DeviceIdSignalId<DeviceProviderIdDeviceId>, SignalRemoteBase> {
        self.device_provider_contexts
            .iter()
            .map(move |(device_provider_id, device_provider_context)| {
                let device_provider_id = *device_provider_id;
                device_provider_context.get_signals_remote_bases().map(
                    move |device_signals_remote_bases| {
                        (device_provider_id, device_signals_remote_bases)
                    },
                )
            })
            .collect::<JoinAll<_>>()
            .await
            .into_iter()
            .flat_map(move |(device_provider_id, device_signals_remote_bases)| {
                device_signals_remote_bases.into_iter().map(
                    move |(device_id_signal_id, signal_remote_base)| {
                        (
                            DeviceIdSignalId::new(
                                DeviceProviderIdDeviceId::new(
                                    device_provider_id,
                                    device_id_signal_id.device_id(),
                                ),
                                device_id_signal_id.signal_id(),
                            ),
                            signal_remote_base,
                        )
                    },
                )
            })
            .collect()
    }

    pub async fn run(&self) -> ! {
        log::trace!("run called");

        let run_device_provider_contexts = self.run_device_provider_contexts();
        pin_mut!(run_device_provider_contexts);
        let mut run_device_provider_contexts = run_device_provider_contexts.fuse();

        let handler_async_bridge_run = self.handler_async_bridge.run(self);
        pin_mut!(handler_async_bridge_run);
        let mut handler_async_bridge_run = handler_async_bridge_run.fuse();

        log::trace!("run starting main select");

        select! {
            _ = run_device_provider_contexts => {
                panic!("run_device_provider_contexts yielded");
            }
            _ = handler_async_bridge_run => {
                panic!("handler_async_bridge_run yielded");
            },
        }
    }
    async fn run_device_provider_contexts(&self) -> ! {
        log::trace!("run_device_provider_contexts called");

        self.device_provider_contexts
            .values()
            .map(|device_provider| device_provider.run())
            .collect::<SelectAllEmptyFutureInfinite<_>>()
            .await;

        panic!("run_device_provider_contexts yielded");
    }

    pub async fn finalize(self) {
        log::trace!("finalize begin");

        Self::finalize_device_provider_contexts(self.device_provider_contexts).await;

        log::trace!("finalize end");
    }
    async fn finalize_device_provider_contexts(
        device_provider_contexts: HashMap<DeviceProviderId, DeviceProviderContext<'p>>
    ) {
        log::trace!("finalize_device_provider_contexts begin");

        device_provider_contexts
            .into_iter()
            .map(|(_, device_provider_context)| device_provider_context.finalize())
            .collect::<JoinAll<_>>()
            .await;

        log::trace!("finalize_device_provider_contexts end");
    }
}
impl<'p> HandlerAsync for DevicePoolProvidersContext<'p> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'_, BoxFuture<'static, Response>> {
        async move {
            match (request.method(), uri_cursor.next_item()) {
                // Device list
                (&Method::GET, ("", None)) => {
                    let device_provider_id_device_ids =
                        self.get_device_provider_id_device_ids().await;

                    async move {
                        Response::ok_json(
                            device_provider_id_device_ids
                                .into_iter()
                                .map(|device_provider_id_device_id| {
                                    device_provider_id_device_id.to_combined()
                                })
                                .collect::<HashSet<_>>(),
                        )
                    }
                    .boxed()
                }
                // Device access
                (_, uri_cursor_next_item) => {
                    let (device_provider_id_device_id_combined_str, uri_cursor_next_item) =
                        match uri_cursor_next_item {
                            (device_provider_id_device_id_combined_str, Some(uri_cursor_next_item)) => {
                                (device_provider_id_device_id_combined_str, uri_cursor_next_item)
                            }
                            _ => return async move { Response::error_404() }.boxed(),
                        };
                    let device_provider_id_device_id_combined: u64 =
                        match device_provider_id_device_id_combined_str.parse() {
                            Ok(device_provider_id_device_id_combined) => device_provider_id_device_id_combined,
                            _ => return async move { Response::error_404() }.boxed(),
                        };
                    let device_provider_id_device_id =
                        match DeviceProviderIdDeviceId::from_combined(device_provider_id_device_id_combined)
                        {
                            Ok(device_provider_id_device_id) => device_provider_id_device_id,
                            Err(_) => return async move { Response::error_404() }.boxed(),
                        };
                    let device_provider = match self
                        .device_provider_contexts
                        .get(&device_provider_id_device_id.device_provider_id())
                    {
                        Some(device_provider) => device_provider,
                        None => return async move { Response::error_404() }.boxed(),
                    };
                    device_provider
                        .web_handle(
                            device_provider_id_device_id.device_id(),
                            request,
                            uri_cursor_next_item,
                        )
                        .await
                }
            }
        }
        .boxed()
    }
}
impl<'p> Handler for DevicePoolProvidersContext<'p> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        self.handler_async_bridge.handle(request, uri_cursor)
    }
}
