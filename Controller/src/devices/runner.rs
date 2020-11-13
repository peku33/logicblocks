use super::{DeviceHandler, Id as DeviceId};
use crate::{
    signals::{
        exchange::{
            connections_requested::Connections as ConnectionsRequested, exchanger::Exchanger,
        },
        Device as SignalsDevice,
    },
    util::tokio_cancelable::ScopedSpawn,
    web::{self, sse_aggregated, uri_cursor},
};
use anyhow::Context;
use futures::{
    future::{BoxFuture, JoinAll},
    FutureExt,
};
use owning_ref::OwningHandle;
use std::collections::HashMap;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

struct RunnerRuntimeDevicesContext<'r, 'd> {
    devices_run_scoped_spawn: Box<[ScopedSpawn<'r, 'd, !>]>,
    exchanger_context: OwningHandle<Box<Exchanger<'d>>, Box<ScopedSpawn<'r, 'd, !>>>,
    devices_gui_summary_sse_aggregated_bus: sse_aggregated::Bus,
}

struct RunnerRuntimeDevices<'d> {
    runtime: Runtime,
    devices_handler: HashMap<DeviceId, DeviceHandler<'d>>,
}

pub struct Runner<'d> {
    runtime_devices_context:
        OwningHandle<Box<RunnerRuntimeDevices<'d>>, Box<RunnerRuntimeDevicesContext<'d, 'd>>>,
}
impl<'d> Runner<'d> {
    pub fn new(
        devices_handler: HashMap<DeviceId, DeviceHandler<'d>>,
        connections_requested: ConnectionsRequested,
    ) -> Self {
        let runtime = RuntimeBuilder::new()
            .enable_all()
            .threaded_scheduler()
            .thread_name("Runner.devices")
            .build()
            .unwrap();

        let runtime_devices_context = OwningHandle::new_with_fn(
            Box::new(RunnerRuntimeDevices {
                runtime,
                devices_handler,
            }),
            |runtime_devices_context_ptr| {
                let runtime_devices_context = unsafe { &*runtime_devices_context_ptr };

                let devices_run_scoped_spawn = runtime_devices_context
                    .devices_handler
                    .values()
                    .map(|device_handler| {
                        ScopedSpawn::new(
                            &runtime_devices_context.runtime,
                            device_handler.device().run(),
                        )
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();

                let exchanger_devices = runtime_devices_context
                    .devices_handler
                    .iter()
                    .map(|(device_id, device_handler)| {
                        (*device_id, device_handler.device().as_signals_device())
                    })
                    .collect::<HashMap<DeviceId, &'d dyn SignalsDevice>>();
                let exchanger = Exchanger::new(exchanger_devices, &connections_requested);
                let exchanger_context =
                    OwningHandle::new_with_fn(Box::new(exchanger), |exchanger_ptr| {
                        let exchanger = unsafe { &*exchanger_ptr };

                        let scoped_spawn = ScopedSpawn::new(
                            &runtime_devices_context.runtime,
                            exchanger.run().boxed(),
                        );
                        Box::new(scoped_spawn)
                    });

                let devices_gui_summary_sse_aggregated_node = sse_aggregated::Node {
                    terminal: None,
                    children: runtime_devices_context
                        .devices_handler
                        .iter()
                        .map(|(device_id, device_handler)| {
                            (
                                sse_aggregated::PathItem::NumberU32(*device_id),
                                device_handler.gui_summary_waker(),
                            )
                        })
                        .collect(),
                };
                let devices_gui_summary_sse_aggregated_bus =
                    sse_aggregated::Bus::new(devices_gui_summary_sse_aggregated_node);

                Box::new(RunnerRuntimeDevicesContext {
                    devices_run_scoped_spawn,
                    exchanger_context,
                    devices_gui_summary_sse_aggregated_bus,
                })
            },
        );

        Self {
            runtime_devices_context,
        }
    }

    pub async fn finalize(mut self) -> HashMap<DeviceId, DeviceHandler<'d>> {
        // Cancel exchanger
        self.runtime_devices_context
            .exchanger_context
            .abort()
            .await
            .unwrap_none();

        // Cancel running devices
        self.runtime_devices_context
            .devices_run_scoped_spawn
            .iter_mut()
            .map(|scoped_spawn| scoped_spawn.abort())
            .collect::<JoinAll<_>>()
            .await
            .into_iter()
            .for_each(|abort_result| abort_result.unwrap_none());

        // Finalize devices
        self.runtime_devices_context
            .as_owner()
            .devices_handler
            .values()
            .map(|device_handler| {
                ScopedSpawn::new(
                    &self.runtime_devices_context.as_owner().runtime,
                    device_handler.device().finalize(),
                )
            })
            .collect::<JoinAll<_>>()
            .await
            .into_iter()
            .for_each(|abort_result| abort_result.unwrap());

        let runtime_devices_context = self.runtime_devices_context.into_owner();

        // Shutdown runtime
        // This is safe, because all join handles are awaited
        runtime_devices_context.runtime.shutdown_background();

        // Return devices
        runtime_devices_context.devices_handler
    }
}
impl<'p> uri_cursor::Handler for Runner<'p> {
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
                                .runtime_devices_context
                                .as_owner()
                                .devices_handler
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
                                .runtime_devices_context
                                .devices_gui_summary_sse_aggregated_bus
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
                    let device_context_run_context = match self
                        .runtime_devices_context
                        .as_owner()
                        .devices_handler
                        .get(&device_id)
                    {
                        Some(device_context_run_context) => device_context_run_context,
                        None => return async move { web::Response::error_404() }.boxed(),
                    };
                    device_context_run_context.handle(request, &*uri_cursor)
                }
                _ => async move { web::Response::error_404() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
