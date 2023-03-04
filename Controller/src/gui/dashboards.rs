use super::fontawesome::Icon;
use crate::{
    devices::{
        self,
        helpers::{DeviceHandle, DeviceHandleErased},
    },
    signals,
    web::{self, uri_cursor},
};
use anyhow::Context;
use futures::future::{BoxFuture, FutureExt};
use serde::Serialize;

// TODO: Make device handles deduplicable
#[derive(Debug)]
pub struct Dashboard<'d> {
    name: String,
    icon: Icon,

    device_handles_erased: Vec<DeviceHandleErased<'d>>,
}
impl<'d> Dashboard<'d> {
    pub fn new(
        name: String,
        icon: Icon,
    ) -> Self {
        let device_handles_erased = Vec::<DeviceHandleErased<'d>>::new();

        Self {
            name,
            icon,

            device_handles_erased,
        }
    }

    pub fn insert<D: devices::Device + signals::Device + 'd>(
        &mut self,
        device_handle: DeviceHandle<'d, D>,
    ) {
        self.insert_erased(device_handle.into_erased())
    }
    pub fn insert_erased(
        &mut self,
        device_handle_erased: DeviceHandleErased<'d>,
    ) {
        self.device_handles_erased.push(device_handle_erased);
    }
}
impl<'d> uri_cursor::Handler for Dashboard<'d> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next("summary", uri_cursor) => match uri_cursor.as_ref() {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::GET => {
                        #[derive(Debug, Serialize)]
                        struct DashboardSummary {
                            name: String,
                            icon: Icon,
                            device_ids: Vec<devices::Id>,
                        }

                        let name = self.name.clone();

                        let icon = self.icon.clone();

                        let device_ids = self
                            .device_handles_erased
                            .iter()
                            .map(|device_handle_erased| device_handle_erased.device_id())
                            .collect::<Vec<_>>();

                        let dashboard_summary = DashboardSummary {
                            name,
                            icon,
                            device_ids,
                        };
                        async move { web::Response::ok_json(dashboard_summary) }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}

#[derive(Debug)]
pub struct Dashboards<'d> {
    dashboards: Vec<Dashboard<'d>>,
}
impl<'d> Dashboards<'d> {
    pub fn new() -> Self {
        let dashboards = Vec::<Dashboard<'d>>::new();

        Self { dashboards }
    }
    pub fn insert(
        &mut self,
        dashboard: Dashboard<'d>,
    ) {
        self.dashboards.push(dashboard);
    }
}
impl<'d> uri_cursor::Handler for Dashboards<'d> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next("summary", uri_cursor) => match uri_cursor.as_ref() {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::GET => {
                        #[derive(Debug, Serialize)]
                        struct DashboardSummary {
                            id: usize,
                            name: String,
                            icon: Icon,
                        }

                        let dashboards_summary = self
                            .dashboards
                            .iter()
                            .enumerate()
                            .map(|(index, dashboard_item)| {
                                let id = index + 1; // to start from 1

                                let name = dashboard_item.name.clone();

                                let icon = dashboard_item.icon.clone();

                                let response_item = DashboardSummary { id, name, icon };

                                response_item
                            })
                            .collect::<Vec<_>>();

                        #[derive(Debug, Serialize)]
                        #[serde(transparent)]
                        struct DashboardsSummary {
                            inner: Vec<DashboardSummary>,
                        }

                        let dashboards_summary = DashboardsSummary {
                            inner: dashboards_summary,
                        };

                        async move { web::Response::ok_json(dashboards_summary) }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            uri_cursor::UriCursor::Next(dashboard_id, uri_cursor) => {
                let dashboard_id: usize = match dashboard_id.parse().context("dashboard_id") {
                    Ok(dashboard_id) => dashboard_id,
                    Err(error) => {
                        return async move { web::Response::error_400_from_error(error) }.boxed()
                    }
                };

                #[allow(clippy::absurd_extreme_comparisons)]
                if dashboard_id <= 0 {
                    return async move { web::Response::error_404() }.boxed();
                }
                let dashboard_index = dashboard_id - 1;

                let dashboard = match self.dashboards.get(dashboard_index) {
                    Some(dashboard) => dashboard,
                    None => return async move { web::Response::error_404() }.boxed(),
                };
                dashboard.handle(request, uri_cursor.as_ref())
            }
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
