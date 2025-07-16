use super::optional::StreamOrPending;
use futures::{
    future::{Fuse, Future, FutureExt, SelectAll as FutureSelectAll},
    stream::{SelectAll as StreamSelectAll, Stream},
};
use std::{pin::Pin, task};

#[derive(Debug)]
pub struct FutureSelectAllOrPending<F>
where
    F: Future + Unpin,
{
    inner: Fuse<FutureSelectAll<F>>,
}
// impl<F> Unpin for FutureSelectAllOrPending<F> where F: Future + Unpin {}
impl<F> FromIterator<F> for FutureSelectAllOrPending<F>
where
    F: Future + Unpin,
{
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        let mut iter = iter.into_iter().peekable();

        let inner = if iter.peek().is_some() {
            FutureSelectAll::from_iter(iter).fuse()
        } else {
            Fuse::terminated()
        };

        Self { inner }
    }
}
impl<F> Future for FutureSelectAllOrPending<F>
where
    F: Future + Unpin,
{
    type Output = <FutureSelectAll<F> as Future>::Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        let inner = unsafe { self.map_unchecked_mut(|self_| &mut self_.inner) };
        inner.poll(cx)
    }
}

#[derive(Debug)]
pub struct StreamSelectAllOrPending<S>
where
    S: Stream + Unpin,
{
    inner: StreamOrPending<StreamSelectAll<S>>,
}
// impl<S> Unpin for StreamSelectAllOrPending<S> where S: Stream + Unpin {}
impl<S> FromIterator<S> for StreamSelectAllOrPending<S>
where
    S: Stream + Unpin,
{
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let mut iter = iter.into_iter().peekable();

        let inner = if iter.peek().is_some() {
            Some(StreamSelectAll::from_iter(iter))
        } else {
            None
        };

        let inner = StreamOrPending::new(inner);

        Self { inner }
    }
}
impl<S> Stream for StreamSelectAllOrPending<S>
where
    S: Stream + Unpin,
{
    type Item = <StreamSelectAll<S> as Stream>::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        unsafe { self.map_unchecked_mut(|self_| &mut self_.inner) }.poll_next(cx)
    }
}
