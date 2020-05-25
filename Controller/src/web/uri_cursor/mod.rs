pub mod handler_async_bridge;
pub mod map_router;

use super::{Request, Response};
use futures::future::BoxFuture;
use std::sync::Arc;

pub trait Handler {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response>;
}

pub trait HandlerAsync {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'_, BoxFuture<'static, Response>>;
}

#[derive(Debug)]
pub struct UriCursor {
    uri: Arc<String>,
    position: usize,
}
impl<'a> UriCursor {
    pub fn new(uri: String) -> Self {
        UriCursor {
            uri: Arc::new(uri),
            position: 0,
        }
    }

    pub fn rest(&self) -> &str {
        &self.uri[self.position..]
    }

    pub fn next_item(&self) -> (&str, Option<UriCursor>) {
        let slash_position = self.rest().find('/');
        match slash_position {
            Some(slash_position) => (
                &self.rest()[..slash_position],
                Some(UriCursor {
                    uri: self.uri.clone(),
                    position: self.position + slash_position + 1,
                }),
            ),
            None => (self.rest(), None),
        }
    }
}

#[cfg(test)]
mod test_uri_cursor {
    use super::UriCursor;

    #[test]
    fn test_1() {
        let uri_cursor = UriCursor::new("item1/thing2/the3".to_owned());

        let (item, uri_cursor) = uri_cursor.next_item();
        assert_eq!(item, "item1");
        let uri_cursor = uri_cursor.unwrap();

        let (item, uri_cursor) = uri_cursor.next_item();
        assert_eq!(item, "thing2");
        let uri_cursor = uri_cursor.unwrap();

        let (item, uri_cursor) = uri_cursor.next_item();
        assert_eq!(item, "the3");
        assert!(uri_cursor.is_none());
    }

    #[test]
    fn test_2() {
        let uri_cursor = UriCursor::new("".to_owned());
        let (item, uri_cursor) = uri_cursor.next_item();
        assert_eq!(item, "");
        assert!(uri_cursor.is_none());
    }

    #[test]
    fn test_3() {
        let uri_cursor = UriCursor::new("/".to_owned());
        let (item, uri_cursor) = uri_cursor.next_item();
        assert_eq!(item, "");
        let uri_cursor = uri_cursor.unwrap();

        let (item, uri_cursor) = uri_cursor.next_item();
        assert_eq!(item, "");
        assert!(uri_cursor.is_none());
    }
}
