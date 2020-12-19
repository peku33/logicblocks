use super::{DeviceHandler, Id as DeviceId};
use crate::{
    signals::{
        exchange::{
            connections_requested::Connections as ConnectionsRequested, exchanger::Exchanger,
        },
        Device as SignalsDevice,
    },
    util::scoped_async::ScopedRuntime,
    web::{self, sse_aggregated, uri_cursor},
};
use anyhow::Context;
use futures::future::{BoxFuture, FutureExt};
use owning_ref::OwningHandle;
use std::collections::HashMap;

struct RunnerContextChildChild<'d> {
    scoped_runtime: ScopedRuntime<(&'d RunnerContextOwner<'d>, &'d RunnerContextChildOwner<'d>)>,
}
struct RunnerContextChildOwner<'d> {
    exchanger: Exchanger<'d>,
    devices_gui_summary_sse_aggregated_bus: sse_aggregated::Bus,
}
struct RunnerContextChild<'d> {
    context: OwningHandle<Box<RunnerContextChildOwner<'d>>, Box<RunnerContextChildChild<'d>>>,
}
struct RunnerContextOwner<'d> {
    devices_handler: HashMap<DeviceId, DeviceHandler<'d>>,
}

pub struct Runner<'d> {
    context: OwningHandle<Box<RunnerContextOwner<'d>>, Box<RunnerContextChild<'d>>>,
}
impl<'d> Runner<'d> {
    pub fn new(
        devices_handler: HashMap<DeviceId, DeviceHandler<'d>>,
        connections_requested: ConnectionsRequested,
    ) -> Self {
        let context = OwningHandle::new_with_fn(
            Box::new(RunnerContextOwner { devices_handler }),
            |context_owner_ptr| {
                let context_owner = unsafe { &*context_owner_ptr };

                // devices_gui_summary_sse_aggregated_bus
                let devices_gui_summary_sse_aggregated_node = sse_aggregated::Node {
                    terminal: None,
                    children: context_owner
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

                // exchanger
                let exchanger_devices = context_owner
                    .devices_handler
                    .iter()
                    .map(|(device_id, device_handler)| {
                        (*device_id, device_handler.device().as_signals_device())
                    })
                    .collect::<HashMap<DeviceId, &'d dyn SignalsDevice>>();
                let exchanger = Exchanger::new(exchanger_devices, &connections_requested);

                let context = OwningHandle::new_with_fn(
                    Box::new(RunnerContextChildOwner {
                        exchanger,
                        devices_gui_summary_sse_aggregated_bus,
                    }),
                    |context_child_owner_ptr| {
                        let context_child_owner = unsafe { &*context_child_owner_ptr };

                        let scoped_runtime = ScopedRuntime::new(
                            (context_owner, context_child_owner),
                            "Devices.runner".to_string(),
                        );

                        // Run devices
                        scoped_runtime.spawn_runnables_object_detached(|(context_owner, _)| {
                            context_owner
                                .devices_handler
                                .values()
                                .filter_map(|device_handler| device_handler.device().as_runnable())
                                .collect::<Box<[_]>>()
                        });

                        // Run exchanger
                        scoped_runtime.spawn_runnable_detached(|(_, context_child_owner)| {
                            &context_child_owner.exchanger
                        });

                        Box::new(RunnerContextChildChild { scoped_runtime })
                    },
                );

                Box::new(RunnerContextChild { context })
            },
        );

        Self { context }
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
                                .context
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
                                .context
                                .context
                                .as_owner()
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
                    let device_context_run_context =
                        match self.context.as_owner().devices_handler.get(&device_id) {
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
