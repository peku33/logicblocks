use super::device_event_stream;
use crate::web::uri_cursor::Handler;
use futures::future::LocalBoxFuture;
use std::cell::RefCell;

pub trait DeviceTrait {
    fn device_class_get(&self) -> &'static str;
    fn device_run<'s>(&'s self) -> Box<dyn RunObjectTrait<'s> + 's>;
    fn device_as_routed_handler(&self) -> Option<&dyn Handler> {
        None
    }
}

pub trait AsDeviceTrait {
    fn as_device_trait(&self) -> &dyn DeviceTrait;
}
impl<T: DeviceTrait> AsDeviceTrait for T {
    fn as_device_trait(&self) -> &dyn DeviceTrait {
        self
    }
}

pub trait RunObjectTrait<'d> {
    fn get_run_future(&self) -> &RefCell<LocalBoxFuture<'d, ()>>;
    fn event_stream_subscribe(&self) -> Option<device_event_stream::Receiver>;
}
