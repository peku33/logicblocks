use futures::stream::BoxStream;
use std::{ffi::OsString, io::Error, path::Path};

#[derive(Debug)]
pub struct Inotify {}
impl Inotify {
    pub fn init() -> Result<Self, Error> {
        unimplemented!();
    }
    pub fn watches(&self) -> Watches {
        unimplemented!();
    }
    pub fn into_event_stream(
        self,
        _buffer: impl AsMut<[u8]> + AsRef<[u8]>,
    ) -> Result<BoxStream<'static, Result<EventOwned, Error>>, Error> {
        unimplemented!();
    }
}

#[derive(Debug)]
pub struct Watches {}
impl Watches {
    pub fn add(
        &mut self,
        _path: impl AsRef<Path>,
        _mask: WatchMask,
    ) -> Result<WatchDescriptor, Error> {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct WatchDescriptor {}

#[derive(Debug)]
pub struct WatchMask {}
impl WatchMask {
    pub const CLOSE_WRITE: Self = Self {};
}

#[derive(Debug)]
pub struct EventOwned {
    pub name: Option<OsString>,
}
