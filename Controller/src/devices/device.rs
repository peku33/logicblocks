use super::device_event_stream;
use crate::web::router::uri_cursor::Handler;
use futures::future::LocalBoxFuture;
use std::cell::RefCell;

pub trait DeviceTrait {
    fn device_class_get(&self) -> &'static str;
    fn device_run<'s>(&'s self) -> Box<dyn RunObjectTrait<'s> + 's>;
    fn device_as_routed_handler(&self) -> Option<&dyn Handler> {
        return None;
    }
}

pub trait RunObjectTrait<'d> {
    fn get_run_future(&self) -> &RefCell<LocalBoxFuture<'d, ()>>;
    fn event_stream_subscribe(&self) -> Option<device_event_stream::Receiver>;
}
