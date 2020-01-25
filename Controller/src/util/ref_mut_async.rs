use futures::future::{Future, LocalBoxFuture};
use futures::stream::{LocalBoxStream, Stream};
use futures::task::{Context, Poll};
use std::cell::RefMut;
use std::pin::Pin;

type FutureInner<'a, 'b, T> = RefMut<'a, LocalBoxFuture<'b, T>>;
pub struct FutureWrapper<'a, 'b, T> {
    inner: FutureInner<'a, 'b, T>,
}
impl<'a, 'b, T> FutureWrapper<'a, 'b, T> {
    pub fn new(inner: FutureInner<'a, 'b, T>) -> Self {
        Self { inner }
    }
}
impl<'a, 'b, T> Future for FutureWrapper<'a, 'b, T> {
    type Output = T;
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Self::Output> {
        self.get_mut().inner.as_mut().poll(cx)
    }
}

type StreamInner<'a, 'b, T> = RefMut<'a, LocalBoxStream<'b, T>>;
pub struct StreamWrapper<'a, 'b, T> {
    inner: StreamInner<'a, 'b, T>,
}
impl<'a, 'b, T> StreamWrapper<'a, 'b, T> {
    pub fn new(inner: StreamInner<'a, 'b, T>) -> Self {
        Self { inner }
    }
}
impl<'a, 'b, T> Stream for StreamWrapper<'a, 'b, T> {
    type Item = T;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}
