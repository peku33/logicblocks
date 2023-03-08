pub mod dahua;
pub mod eaton;
pub mod gui_summary;
pub mod helpers;
pub mod hikvision;
pub mod houseblocks;
pub mod runner;
pub mod soft;

use crate::{
    signals,
    util::{
        async_flag,
        runtime::{Exited, Runnable},
    },
    web::{self, uri_cursor},
};
use async_trait::async_trait;
use futures::future::{BoxFuture, FutureExt};
use serde::{de::DeserializeOwned, Serialize};
use std::{borrow::Cow, fmt};

pub type Id = u32;

pub trait Configuration = Serialize + DeserializeOwned;
pub trait State = Serialize + DeserializeOwned;

pub trait Device: Send + Sync + fmt::Debug {
    fn class(&self) -> Cow<'static, str>;

    fn as_runnable(&self) -> &dyn Runnable;
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase;
    fn as_gui_summary_device_base(&self) -> Option<&dyn gui_summary::DeviceBase> {
        None
    }
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
        None
    }
}

#[derive(Debug)]
pub struct DeviceWrapper<'d> {
    name: String,
    device: Box<dyn Device + 'd>,
}
impl<'d> DeviceWrapper<'d> {
    pub fn new(
        name: String,
        device: Box<dyn Device + 'd>,
    ) -> Self {
        Self { name, device }
    }

    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn device(&self) -> &dyn Device {
        &*self.device as &dyn Device
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.device.as_runnable().run(exit_flag).await
    }

    pub fn close(self) -> Box<dyn Device + 'd> {
        self.device
    }
}
#[async_trait]
impl<'d> Runnable for DeviceWrapper<'d> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
impl<'d> uri_cursor::Handler for DeviceWrapper<'d> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Terminal => match *request.method() {
                http::Method::GET => {
                    #[derive(Debug, Serialize)]
                    struct DeviceData {
                        name: String,
                        class: Cow<'static, str>,
                    }

                    let name = self.name().clone();
                    let class = self.device().class();

                    let device_data = DeviceData { name, class };

                    async move { web::Response::ok_json(device_data) }.boxed()
                }
                _ => async move { web::Response::error_405() }.boxed(),
            },
            uri_cursor::UriCursor::Next("gui-summary", uri_cursor) => {
                match self.device().as_gui_summary_device_base() {
                    Some(gui_summary_device_base) => match uri_cursor.as_ref() {
                        uri_cursor::UriCursor::Terminal => match *request.method() {
                            http::Method::GET => {
                                let value = gui_summary_device_base.value();
                                async move { web::Response::ok_json(value) }.boxed()
                            }
                            _ => async move { web::Response::error_405() }.boxed(),
                        },
                        _ => async move { web::Response::error_404() }.boxed(),
                    },
                    None => async move { web::Response::error_404() }.boxed(),
                }
            }
            uri_cursor::UriCursor::Next("device", uri_cursor) => {
                match self.device().as_web_handler() {
                    Some(handler) => handler.handle(request, uri_cursor),
                    None => async move { web::Response::error_404() }.boxed(),
                }
            }
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
