use futures::future::Future;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct DerefAsyncFuture<D>
where
    D: DerefMut,
    D::Target: Future,
    <D::Target as Future>::Output: Send + 'static,
{
    inner: D,
}
impl<D> DerefAsyncFuture<D>
where
    D: DerefMut,
    D::Target: Future,
    <D::Target as Future>::Output: Send + 'static,
{
    pub fn new(inner: D) -> Self {
        Self { inner }
    }
}
impl<D> Future for DerefAsyncFuture<D>
where
    D: DerefMut,
    D::Target: Future,
    <D::Target as Future>::Output: Send + 'static,
{
    type Output = <D::Target as Future>::Output;
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Self::Output> {
        let inner_pin = unsafe { self.map_unchecked_mut(|self_| self_.inner.deref_mut()) };
        inner_pin.poll(cx)
    }
}
