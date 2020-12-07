use futures::stream::BoxStream;
use std::{ffi::OsString, io::Error, path::Path};

#[derive(Debug)]
pub struct EventOwned {
    pub name: Option<OsString>,
}
pub struct Inotify {}
impl Inotify {
    pub fn init() -> Result<Self, Error> {
        unimplemented!();
    }
    pub fn add_watch(
        &mut self,
        _path: &Path,
        _watch_mask: usize,
    ) -> Result<(), Error> {
        unimplemented!();
    }
    pub fn event_stream(
        &mut self,
        _buffer: impl AsMut<[u8]> + AsRef<[u8]>,
    ) -> Result<BoxStream<Result<EventOwned, Error>>, Error> {
        unimplemented!();
    }
}
pub struct WatchMask {}
impl WatchMask {
    pub const CLOSE_WRITE: usize = 0;
}
