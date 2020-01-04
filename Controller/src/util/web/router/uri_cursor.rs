use crate::util::web::{Handler as WebHandler, Request, Response};
use futures::future::{ready, BoxFuture, FutureExt};
use http::StatusCode;
use std::collections::HashMap;

pub trait Handler {
    fn handle(
        &self,
        request: &Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response>;
}

pub struct Root<'r> {
    parent: &'r dyn Handler,
}
impl<'r> Root<'r> {
    pub fn new(parent: &'r dyn Handler) -> Self {
        return Self { parent };
    }
}
impl<'r> WebHandler for Root<'r> {
    fn handle(
        &self,
        request: &Request,
    ) -> BoxFuture<'static, Response> {
        let path = request.http_parts.uri.path();
        if path.len() <= 0 || path.chars().next().unwrap() != '/' {
            return ready(Response::error(StatusCode::BAD_REQUEST)).boxed();
        }
        let path = &path[1..];
        let uri_cursor = UriCursor::new(path);
        return self.parent.handle(request, uri_cursor);
    }
}

pub struct Map<'a, 'b> {
    map: HashMap<&'a str, &'b dyn Handler>,
}
impl<'a, 'b> Map<'a, 'b> {
    // pub fn new<I: Iterator<Item = (&'a str, &'b dyn Handler)>>(i: I) -> Self {
    //     return Self { map: i.collect() };
    // }
    pub fn new(map: HashMap<&'a str, &'b dyn Handler>) -> Self {
        return Self { map };
    }
}
impl Handler for Map<'_, '_> {
    fn handle(
        &self,
        request: &Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        let (next_item, uri_cursor) = uri_cursor.next_item();
        let uri_cursor = match uri_cursor {
            Some(uri_cursor) => uri_cursor,
            None => return ready(Response::error_404()).boxed(),
        };
        let item = self.map.get(next_item);
        match item {
            Some(next_routed_handler) => return next_routed_handler.handle(request, uri_cursor),
            None => return ready(Response::error_404()).boxed(),
        }
    }
}

#[derive(Debug)]
pub struct UriCursor<'a> {
    uri_left: &'a str,
}
impl<'a> UriCursor<'a> {
    pub fn new(uri: &'a str) -> Self {
        return UriCursor { uri_left: uri };
    }

    pub fn rest(&self) -> &'a str {
        return self.uri_left;
    }

    pub fn next_item(&self) -> (&'a str, Option<UriCursor<'a>>) {
        let slash_position = self.uri_left.find('/');
        return match slash_position {
            Some(slash_position) => (
                &self.uri_left[..slash_position],
                Some(UriCursor::new(&self.uri_left[slash_position + 1..])),
            ),
            None => (self.uri_left, None),
        };
    }
}

#[cfg(test)]
mod test_uri_cursor {
    use super::UriCursor;

    #[test]
    fn test_1() {
        let uri_cursor = UriCursor::new("item1/thing2/the3");

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
        let uri_cursor = UriCursor::new("");
        let (item, uri_cursor) = uri_cursor.next_item();
        assert_eq!(item, "");
        assert!(uri_cursor.is_none());
    }

    #[test]
    fn test_3() {
        let uri_cursor = UriCursor::new("/");
        let (item, uri_cursor) = uri_cursor.next_item();
        assert_eq!(item, "");
        let uri_cursor = uri_cursor.unwrap();

        let (item, uri_cursor) = uri_cursor.next_item();
        assert_eq!(item, "");
        assert!(uri_cursor.is_none());
    }
}
