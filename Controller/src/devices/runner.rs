use super::{DeviceContext, Id as DeviceId};
use crate::{
    signals::{
        exchange::{
            connections_requested::Connections as ConnectionsRequested, exchanger::Exchanger,
        },
        Device as SignalsDevice,
    },
    util::select_all_empty::SelectAllEmptyFutureInfinite,
    web::{self, sse_aggregated, uri_cursor},
};
use anyhow::Context;
use futures::{future::BoxFuture, pin_mut, select, FutureExt};
use owning_ref::OwningHandle;
use std::collections::HashMap;

struct RunnerInner<'d> {
    exchanger: Exchanger<'d>,
    devices_gui_summary_sse_aggregated_bus: sse_aggregated::Bus,
}

pub struct Runner<'d> {
    inner: OwningHandle<Box<HashMap<DeviceId, DeviceContext<'d>>>, Box<RunnerInner<'d>>>,
}
impl<'d> Runner<'d> {
    pub fn new(
        device_contexts: HashMap<DeviceId, DeviceContext<'d>>,
        connections_requested: ConnectionsRequested,
    ) -> Self {
        let inner =
            OwningHandle::new_with_fn(Box::new(device_contexts), |device_contexts_box_ptr| {
                let device_contexts = unsafe { &*device_contexts_box_ptr };

                let exchanger_devices = device_contexts
                    .iter()
                    .map(|(device_id, device_context)| {
                        (*device_id, device_context.device().as_signals_device())
                    })
                    .collect::<HashMap<DeviceId, &'d dyn SignalsDevice>>();
                let exchanger = Exchanger::new(exchanger_devices, &connections_requested);

                let devices_gui_summary_sse_aggregated_node = sse_aggregated::Node {
                    terminal: None,
                    children: device_contexts
                        .iter()
                        .map(|(device_id, device_context)| {
                            (
                                sse_aggregated::PathItem::NumberU32(*device_id),
                                device_context.gui_summary_waker(),
                            )
                        })
                        .collect(),
                };
                let devices_gui_summary_sse_aggregated_bus =
                    sse_aggregated::Bus::new(devices_gui_summary_sse_aggregated_node);

                let inner = RunnerInner {
                    exchanger,
                    devices_gui_summary_sse_aggregated_bus,
                };
                Box::new(inner)
            });

        Self { inner }
    }

    pub async fn run(&self) -> ! {
        let mut device_contexts_runner = self
            .inner
            .as_owner()
            .values()
            .map(|device_context| device_context.run())
            .collect::<SelectAllEmptyFutureInfinite<_>>();

        let exchanger_runner = self.inner.exchanger.run();
        let exchanger_runner = exchanger_runner.fuse();
        pin_mut!(exchanger_runner);

        select! {
            _ = device_contexts_runner => panic!("device_contexts_runner yielded"),
            _ = exchanger_runner => panic!("exchanger_runner yielded"),
        }
    }

    pub fn close(self) -> HashMap<DeviceId, DeviceContext<'d>> {
        *self.inner.into_owner()
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
                            let device_ids =
                                self.inner.as_owner().keys().copied().collect::<Vec<_>>();
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
                    let device_context_run_context = match self.inner.as_owner().get(&device_id) {
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
