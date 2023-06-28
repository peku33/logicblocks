pub mod map_router;

use super::{Request, Response};
use futures::future::BoxFuture;

pub trait Handler {
    fn handle(
        &self,
        request: Request,
        uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response>;
}

#[derive(Debug)]
pub enum UriCursor<'p> {
    Terminal,
    Next(&'p str, Box<UriCursor<'p>>),
}
impl<'p> UriCursor<'p> {
    pub fn new(path: &'p str) -> Self {
        match path.find('/') {
            Some(slash_position) => UriCursor::Next(
                &path[..slash_position],
                Box::new(UriCursor::new(&path[slash_position + 1..])),
            ),
            None => UriCursor::Next(path, Box::new(UriCursor::Terminal)),
        }
    }

    pub fn as_last(&self) -> Option<&str> {
        let (url, next) = match self {
            Self::Terminal => return None,
            Self::Next(url, next) => (url, next),
        };
        if let Self::Next(_, _) = next.as_ref() {
            return None;
        }
        Some(url)
    }
}
