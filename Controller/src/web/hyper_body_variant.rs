use http::HeaderMap;
use http_body::SizeHint;
use hyper::body::{Buf, HttpBody};
use std::error::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

// Common boxed types
pub type DataVariantBox = Box<dyn Buf + Sync + Send>;
pub type ErrorVariantBox = Box<dyn Error + Sync + Send>;

// Primary variant type
pub struct HttpBodyVariant {
    inner: Pin<Box<dyn HttpBody<Data = DataVariantBox, Error = ErrorVariantBox> + Sync + Send>>,
}
impl HttpBodyVariant {
    pub fn from<T>(inner: T) -> Self
    where
        T: HttpBody + Sync + Send + 'static,
        T::Data: Buf + Sync + Send,
        T::Error: Error + Sync + Send,
    {
        Self {
            inner: Box::pin(HttpBodyVariantInner::new(inner)),
        }
    }
}
impl HttpBody for HttpBodyVariant {
    type Data = DataVariantBox;
    type Error = ErrorVariantBox;

    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let inner_self = unsafe { self.map_unchecked_mut(|self_| &mut self_.inner) };
        inner_self.poll_data(cx)
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
        let inner_self = unsafe { self.map_unchecked_mut(|self_| &mut self_.inner) };
        inner_self.poll_trailers(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

// Compatibility type
struct HttpBodyVariantInner<T>
where
    T: HttpBody + Sync + Send + 'static,
    T::Data: Buf + Sync + Send,
    T::Error: Error + Sync + Send,
{
    inner: T,
}
impl<T> HttpBodyVariantInner<T>
where
    T: HttpBody + Sync + Send + 'static,
    T::Data: Buf + Sync + Send,
    T::Error: Error + Sync + Send,
{
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}
impl<T> HttpBody for HttpBodyVariantInner<T>
where
    T: HttpBody + Sync + Send + 'static,
    T::Data: Buf + Sync + Send,
    T::Error: Error + Sync + Send,
{
    type Data = DataVariantBox;
    type Error = ErrorVariantBox;

    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let inner_self = unsafe { self.map_unchecked_mut(|self_| &mut self_.inner) };
        inner_self
            .poll_data(cx)
            .map_ok(|o| -> Self::Data { Box::new(o) })
            .map_err(|e| e.into())
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
        let inner_self = unsafe { self.map_unchecked_mut(|self_| &mut self_.inner) };
        inner_self.poll_trailers(cx).map_err(|e| e.into())
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}
