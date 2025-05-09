#![allow(clippy::drop_non_drop)] // TODO: something in self_referencing
#![allow(clippy::too_many_arguments)] // TODO: something in self_referencing

use super::{DeviceWrapper, Id as DeviceId};
use crate::{
    modules::module_path::ModulePath,
    signals::{
        DeviceBaseRef as SignalsDeviceBaseRef,
        exchanger::{ConnectionRequested, Exchanger},
    },
    util::{
        drop_guard::DropGuard,
        runtime::{Runtime, RuntimeScopeRunnable},
    },
    web::{self, sse_topic, uri_cursor},
};
use anyhow::{Context, Error};
use futures::future::{BoxFuture, FutureExt, JoinAll};
use once_cell::sync::Lazy;
use ouroboros::self_referencing;
use std::{collections::HashMap, mem::ManuallyDrop};

#[self_referencing]
#[derive(Debug)]
struct RunnerInner<'d> {
    runtime: Runtime,
    device_wrappers_by_id: HashMap<DeviceId, DeviceWrapper<'d>>,

    #[borrows(runtime, device_wrappers_by_id)]
    #[not_covariant]
    devices_wrapper_runtime_scope_runnable:
        ManuallyDrop<Box<[RuntimeScopeRunnable<'this, 'this, DeviceWrapper<'d>>]>>,

    #[borrows(device_wrappers_by_id)]
    #[covariant]
    exchanger: Exchanger<'this>,

    #[borrows(runtime, exchanger)]
    #[not_covariant]
    exchanger_runtime_scope_runnable:
        ManuallyDrop<RuntimeScopeRunnable<'this, 'this, Exchanger<'this>>>,

    #[borrows(device_wrappers_by_id)]
    #[covariant]
    devices_gui_summary_sse_root_node: sse_topic::Node<'this>,

    #[borrows(devices_gui_summary_sse_root_node)]
    #[covariant]
    devices_gui_summary_sse_responder: sse_topic::Responder<'this>,

    #[borrows(runtime, devices_gui_summary_sse_responder)]
    #[not_covariant]
    devices_gui_summary_sse_responder_runtime_scope_runnable:
        ManuallyDrop<RuntimeScopeRunnable<'this, 'this, sse_topic::Responder<'this>>>,
}

#[derive(Debug)]
pub struct Runner<'d> {
    inner: RunnerInner<'d>,

    drop_guard: DropGuard,
}
impl<'d> Runner<'d> {
    fn module_path() -> &'static ModulePath {
        static MODULE_PATH: Lazy<ModulePath> =
            Lazy::new(|| ModulePath::new(&["devices", "runner"]));
        &MODULE_PATH
    }

    pub fn new(
        device_wrappers_by_id: HashMap<DeviceId, DeviceWrapper<'d>>,
        connections_requested: &[ConnectionRequested],
    ) -> Result<Self, Error> {
        let runtime = Runtime::new(Self::module_path(), 4, 4);

        let inner = RunnerInner::try_new(
            runtime,
            device_wrappers_by_id,
            |runtime, device_wrappers_by_id| -> Result<_, Error> {
                let devices_wrapper_runtime_scope_runnable = device_wrappers_by_id
                    .values()
                    .map(|device_wrapper| RuntimeScopeRunnable::new(runtime, device_wrapper))
                    .collect::<Box<[_]>>();
                let devices_wrapper_runtime_scope_runnable =
                    ManuallyDrop::new(devices_wrapper_runtime_scope_runnable);
                Ok(devices_wrapper_runtime_scope_runnable)
            },
            |device_wrappers_by_id| -> Result<_, Error> {
                let exchanger_devices = device_wrappers_by_id
                    .iter()
                    .map(|(device_id, device_wrapper)| {
                        let device_id = *device_id;

                        let signals_device_base = device_wrapper.device().as_signals_device_base();
                        let signals_device_base =
                            SignalsDeviceBaseRef::from_device_base(signals_device_base);

                        (device_id, signals_device_base)
                    })
                    .collect::<HashMap<_, _>>();
                let exchanger =
                    Exchanger::new(&exchanger_devices, connections_requested).context("new")?;
                Ok(exchanger)
            },
            |runtime, exchanger| -> Result<_, Error> {
                let exchanger_runtime_scope_runnable =
                    RuntimeScopeRunnable::new(runtime, exchanger);
                let exchanger_runtime_scope_runnable =
                    ManuallyDrop::new(exchanger_runtime_scope_runnable);
                Ok(exchanger_runtime_scope_runnable)
            },
            |device_wrappers_by_id| -> Result<_, Error> {
                let devices_gui_summary_sse_root_node = sse_topic::Node::new(
                    None,
                    device_wrappers_by_id
                        .iter()
                        .map(|(device_id, device_wrapper)| {
                            let signal = device_wrapper.device().as_gui_summary_device_base().map(
                                |gui_summary_device_base| {
                                    gui_summary_device_base.waker().as_signal()
                                },
                            );

                            let topic = sse_topic::Topic::Number(*device_id as usize);
                            let node = sse_topic::Node::new(signal, HashMap::new());

                            (topic, node)
                        })
                        .collect::<HashMap<_, _>>(),
                );
                Ok(devices_gui_summary_sse_root_node)
            },
            |devices_gui_summary_sse_root_node| -> Result<_, Error> {
                let devices_gui_summary_sse_responder =
                    sse_topic::Responder::new(devices_gui_summary_sse_root_node);
                Ok(devices_gui_summary_sse_responder)
            },
            |runtime, devices_gui_summary_sse_responder| -> Result<_, Error> {
                let devices_gui_summary_sse_responder_runtime_scope_runnable =
                    RuntimeScopeRunnable::new(runtime, devices_gui_summary_sse_responder);
                let devices_gui_summary_sse_responder_runtime_scope_runnable =
                    ManuallyDrop::new(devices_gui_summary_sse_responder_runtime_scope_runnable);
                Ok(devices_gui_summary_sse_responder_runtime_scope_runnable)
            },
        )
        .context("try_new")?;

        let drop_guard = DropGuard::new();

        Ok(Self { inner, drop_guard })
    }
    pub async fn finalize(mut self) -> HashMap<DeviceId, DeviceWrapper<'d>> {
        let devices_gui_summary_sse_responder_runtime_scope_runnable = self
            .inner
            .with_devices_gui_summary_sse_responder_runtime_scope_runnable_mut(
                |devices_gui_summary_sse_responder_runtime_scope_runnable| unsafe {
                    ManuallyDrop::take(devices_gui_summary_sse_responder_runtime_scope_runnable)
                },
            );
        devices_gui_summary_sse_responder_runtime_scope_runnable
            .finalize()
            .await;

        let exchanger_runtime_scope_runnable = self
            .inner
            .with_exchanger_runtime_scope_runnable_mut(|exchanger_runtime_scope_runnable| unsafe {
                ManuallyDrop::take(exchanger_runtime_scope_runnable)
            });
        exchanger_runtime_scope_runnable.finalize().await;

        let devices_wrapper_runtime_scope_runnable =
            self.inner.with_devices_wrapper_runtime_scope_runnable_mut(
                |devices_wrapper_runtime_scope_runnable| unsafe {
                    ManuallyDrop::take(devices_wrapper_runtime_scope_runnable)
                },
            );
        devices_wrapper_runtime_scope_runnable
            .into_iter()
            .map(|device_wrapper_runtime_scope_runnable| {
                device_wrapper_runtime_scope_runnable.finalize()
            })
            .collect::<JoinAll<_>>()
            .await;

        self.drop_guard.set();

        let inner_heads = self.inner.into_heads();
        inner_heads.device_wrappers_by_id
    }
}
impl uri_cursor::Handler for Runner<'_> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next("devices", uri_cursor) => match uri_cursor.as_ref() {
                uri_cursor::UriCursor::Next("list", uri_cursor) => match uri_cursor.as_ref() {
                    uri_cursor::UriCursor::Terminal => match *request.method() {
                        http::Method::GET => {
                            let device_ids = self
                                .inner
                                .borrow_device_wrappers_by_id()
                                .keys()
                                .copied()
                                .collect::<Box<[_]>>();
                            async { web::Response::ok_json(device_ids) }.boxed()
                        }
                        _ => async { web::Response::error_405() }.boxed(),
                    },
                    _ => async { web::Response::error_404() }.boxed(),
                },
                uri_cursor::UriCursor::Next("gui-summary-sse", uri_cursor) => self
                    .inner
                    .borrow_devices_gui_summary_sse_responder()
                    .handle(request, uri_cursor),
                uri_cursor::UriCursor::Next(device_id_str, uri_cursor) => {
                    let device_id: DeviceId = match device_id_str.parse().context("device_id") {
                        Ok(device_id) => device_id,
                        Err(error) => {
                            return async { web::Response::error_400_from_error(error) }.boxed();
                        }
                    };
                    let device_wrapper =
                        match self.inner.borrow_device_wrappers_by_id().get(&device_id) {
                            Some(device_wrapper) => device_wrapper,
                            None => return async { web::Response::error_404() }.boxed(),
                        };
                    device_wrapper.handle(request, uri_cursor.as_ref())
                }
                _ => async { web::Response::error_404() }.boxed(),
            },
            _ => async { web::Response::error_404() }.boxed(),
        }
    }
}
