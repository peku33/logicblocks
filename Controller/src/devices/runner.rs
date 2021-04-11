use super::{DeviceWrapper, Id as DeviceId};
use crate::{
    signals::exchange::{
        connections_requested::Connections as ConnectionsRequested, exchanger::Exchanger,
    },
    util::runtime::{FinalizeGuard, Runtime, RuntimeScopeRunnable},
    web::{self, sse_aggregated, uri_cursor},
};
use anyhow::Context;
use futures::future::{BoxFuture, FutureExt, JoinAll};
use ouroboros::self_referencing;
use std::{
    collections::HashMap,
    mem::{transmute, ManuallyDrop},
};

#[self_referencing(chain_hack)]
struct RunnerInner<'d> {
    runtime: Box<Runtime>,
    device_wrappers: Box<HashMap<DeviceId, DeviceWrapper<'d>>>,

    #[borrows(device_wrappers)]
    #[not_covariant]
    exchanger: Box<Exchanger<'this>>,

    // #[borrows(device_wrappers)]
    // #[not_covariant]
    devices_gui_summary_sse_aggregated_bus: sse_aggregated::Bus,

    #[borrows(runtime, device_wrappers)]
    #[not_covariant]
    devices_wrapper_runtime_scope_runnable:
        ManuallyDrop<Box<[RuntimeScopeRunnable<'this, 'this, DeviceWrapper<'d>>]>>,

    #[borrows(runtime, exchanger)]
    #[not_covariant]
    exchanger_runtime_scope_runnable:
        ManuallyDrop<RuntimeScopeRunnable<'this, 'this, Exchanger<'this>>>,
}

pub struct Runner<'d> {
    inner: RunnerInner<'d>,
    finalize_guard: FinalizeGuard,
}
impl<'d> Runner<'d> {
    pub fn new(
        device_wrappers: HashMap<DeviceId, DeviceWrapper<'d>>,
        connections_requested: &ConnectionsRequested,
    ) -> Self {
        let runtime = Runtime::new("devices", 4, 4);
        let runtime = Box::new(runtime);

        let device_wrappers = Box::new(device_wrappers);

        let devices_gui_summary_sse_aggregated_node = sse_aggregated::Node {
            terminal: None,
            children: device_wrappers
                .iter()
                .map(|(device_id, device_wrapper)| {
                    (
                        sse_aggregated::PathItem::NumberU32(*device_id),
                        device_wrapper.gui_summary_waker(),
                    )
                })
                .collect(),
        };
        let devices_gui_summary_sse_aggregated_bus =
            sse_aggregated::Bus::new(devices_gui_summary_sse_aggregated_node);

        let inner = RunnerInnerBuilder {
            runtime,
            device_wrappers,
            exchanger_builder: |device_wrappers| {
                let exchanger_devices = device_wrappers
                    .iter()
                    .map(|(device_id, device_wrapper)| {
                        let signals_device = device_wrapper.as_signals_device();
                        let device_id = *device_id;
                        (device_id, signals_device)
                    })
                    .collect::<HashMap<_, _>>();
                let exchanger = Exchanger::new(exchanger_devices, connections_requested);
                Box::new(exchanger)
            },
            devices_gui_summary_sse_aggregated_bus,
            devices_wrapper_runtime_scope_runnable_builder: |runtime, device_wrappers| {
                let devices_wrapper_runtime_scope_runnable = device_wrappers
                    .values()
                    .map(|device_wrapper| RuntimeScopeRunnable::new(runtime, device_wrapper))
                    .collect::<Box<[_]>>();
                let devices_wrapper_runtime_scope_runnable =
                    ManuallyDrop::new(devices_wrapper_runtime_scope_runnable);
                devices_wrapper_runtime_scope_runnable
            },
            exchanger_runtime_scope_runnable_builder: |runtime, exchanger| {
                let exchanger_runtime_scope_runnable =
                    RuntimeScopeRunnable::new(runtime, exchanger);
                let exchanger_runtime_scope_runnable =
                    ManuallyDrop::new(exchanger_runtime_scope_runnable);
                exchanger_runtime_scope_runnable
            },
        }
        .build();

        let finalize_guard = FinalizeGuard::new();

        Self {
            inner,
            finalize_guard,
        }
    }
    pub async fn finalize(mut self) -> HashMap<DeviceId, DeviceWrapper<'d>> {
        let exchanger_runtime_scope_runnable = self
            .inner
            .with_exchanger_runtime_scope_runnable_mut(|exchanger_runtime_scope_runnable| {
                let exchanger_runtime_scope_runnable = unsafe {
                    transmute::<
                        &mut ManuallyDrop<RuntimeScopeRunnable<'_, '_, Exchanger<'_>>>,
                        &mut ManuallyDrop<
                            RuntimeScopeRunnable<'static, 'static, Exchanger<'static>>,
                        >,
                    >(exchanger_runtime_scope_runnable)
                };
                let exchanger_runtime_scope_runnable =
                    unsafe { ManuallyDrop::take(exchanger_runtime_scope_runnable) };
                exchanger_runtime_scope_runnable
            });
        exchanger_runtime_scope_runnable.finalize().await;

        let devices_wrapper_runtime_scope_runnable =
            self.inner.with_devices_wrapper_runtime_scope_runnable_mut(
                |devices_wrapper_runtime_scope_runnable| {
                    let devices_wrapper_runtime_scope_runnable = unsafe {
                        transmute::<
                            &mut ManuallyDrop<
                                Box<[RuntimeScopeRunnable<'_, '_, DeviceWrapper<'_>>]>,
                            >,
                            &mut ManuallyDrop<
                                Box<
                                    [RuntimeScopeRunnable<
                                        'static,
                                        'static,
                                        DeviceWrapper<'static>,
                                    >],
                                >,
                            >,
                        >(devices_wrapper_runtime_scope_runnable)
                    };
                    let devices_wrapper_runtime_scope_runnable =
                        unsafe { ManuallyDrop::take(devices_wrapper_runtime_scope_runnable) };
                    devices_wrapper_runtime_scope_runnable
                },
            );
        devices_wrapper_runtime_scope_runnable
            .into_vec()
            .into_iter()
            .map(|device_wrapper_runtime_scope_runnable| {
                device_wrapper_runtime_scope_runnable.finalize()
            })
            .collect::<JoinAll<_>>()
            .await;

        self.finalize_guard.finalized();

        let inner_heads = self.inner.into_heads();
        *inner_heads.device_wrappers
    }
}
impl<'d> uri_cursor::Handler for Runner<'d> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next("devices", uri_cursor) => match &**uri_cursor {
                uri_cursor::UriCursor::Next("list", uri_cursor) => match **uri_cursor {
                    uri_cursor::UriCursor::Terminal => match *request.method() {
                        http::Method::GET => {
                            let device_ids = self
                                .inner
                                .borrow_device_wrappers()
                                .keys()
                                .copied()
                                .collect::<Vec<_>>();
                            async move { web::Response::ok_json(device_ids) }.boxed()
                        }
                        _ => async move { web::Response::error_405() }.boxed(),
                    },
                    _ => async move { web::Response::error_404() }.boxed(),
                },
                uri_cursor::UriCursor::Next("gui-summary-events", uri_cursor) => match **uri_cursor
                {
                    uri_cursor::UriCursor::Terminal => match *request.method() {
                        http::Method::GET => {
                            let sse_stream = self
                                .inner
                                .borrow_devices_gui_summary_sse_aggregated_bus()
                                .sse_stream();
                            async move { web::Response::ok_sse_stream(sse_stream) }.boxed()
                        }
                        _ => async move { web::Response::error_405() }.boxed(),
                    },
                    _ => async move { web::Response::error_404() }.boxed(),
                },
                uri_cursor::UriCursor::Next(device_id_str, uri_cursor) => {
                    let device_id: DeviceId = match device_id_str.parse().context("device_id") {
                        Ok(device_id) => device_id,
                        Err(error) => {
                            return async move { web::Response::error_400_from_error(error) }
                                .boxed()
                        }
                    };
                    let device_wrapper = match self.inner.borrow_device_wrappers().get(&device_id) {
                        Some(device_wrapper) => device_wrapper,
                        None => return async move { web::Response::error_404() }.boxed(),
                    };
                    device_wrapper.handle(request, &*uri_cursor)
                }
                _ => async move { web::Response::error_404() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
