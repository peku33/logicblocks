use futures::{
    future::{FusedFuture, Future},
    stream::FusedStream,
    task::{Context, Poll},
    Stream,
};
use std::{ops::DerefMut, pin::Pin};

pub struct DerefFuture<D>
where
    D: DerefMut,
    D::Target: Future,
    <D::Target as Future>::Output: Send + 'static,
{
    inner: D,
}
impl<D> DerefFuture<D>
where
    D: DerefMut,
    D::Target: Future,
    <D::Target as Future>::Output: Send + 'static,
{
    pub fn new(inner: D) -> Self {
        Self { inner }
    }
}
impl<D> Future for DerefFuture<D>
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
impl<D> FusedFuture for DerefFuture<D>
where
    D: DerefMut,
    D::Target: Future,
    D::Target: FusedFuture,
    <D::Target as Future>::Output: Send + 'static,
{
    fn is_terminated(&self) -> bool {
        self.inner.deref().is_terminated()
    }
}

pub struct DerefStream<D>
where
    D: DerefMut,
    D::Target: Stream,
    <D::Target as Stream>::Item: Send + 'static,
{
    inner: D,
}
impl<D> DerefStream<D>
where
    D: DerefMut,
    D::Target: Stream,
    <D::Target as Stream>::Item: Send + 'static,
{
    pub fn new(inner: D) -> Self {
        Self { inner }
    }
}
impl<D> Stream for DerefStream<D>
where
    D: DerefMut,
    D::Target: Stream,
    <D::Target as Stream>::Item: Send + 'static,
{
    type Item = <D::Target as Stream>::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let inner = unsafe { self.map_unchecked_mut(|self_| self_.inner.deref_mut()) };
        inner.poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<D> FusedStream for DerefStream<D>
where
    D: DerefMut,
    D::Target: Stream,
    D::Target: FusedStream,
    <D::Target as Stream>::Item: Send + 'static,
{
    fn is_terminated(&self) -> bool {
        self.inner.deref().is_terminated()
    }
}
