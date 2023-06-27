use futures::stream::{Fuse, FusedStream, Stream, StreamExt};
use std::{
    mem::replace,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct ReadyChunksDynamic<S>
where
    S: Stream,
{
    inner: Fuse<S>,
    buffer: Vec<S::Item>,
}
impl<S> ReadyChunksDynamic<S>
where
    S: Stream,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner: inner.fuse(),
            buffer: Vec::<S::Item>::new(),
        }
    }
}
impl<S> Stream for ReadyChunksDynamic<S>
where
    S: Stream,
{
    type Item = Box<[S::Item]>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        loop {
            match unsafe { Pin::new_unchecked(&mut self_.inner) }
                .as_mut()
                .poll_next(cx)
            {
                Poll::Pending => {
                    if self_.buffer.is_empty() {
                        return Poll::Pending;
                    } else {
                        let buffer = replace(&mut self_.buffer, Vec::<S::Item>::new());
                        return Poll::Ready(Some(buffer.into_boxed_slice()));
                    }
                }
                Poll::Ready(Some(item)) => {
                    self_.buffer.push(item);
                }
                Poll::Ready(None) => {
                    if self_.buffer.is_empty() {
                        return Poll::Ready(None);
                    } else {
                        let buffer = replace(&mut self_.buffer, Vec::<S::Item>::new());
                        return Poll::Ready(Some(buffer.into_boxed_slice()));
                    }
                }
            }
        }
    }
}
impl<S> FusedStream for ReadyChunksDynamic<S>
where
    S: Stream,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated() && self.buffer.is_empty()
    }
}

pub trait ReadyChunksDynamicExt: Stream {
    fn ready_chunks_dynamic(self) -> ReadyChunksDynamic<Self>
    where
        Self: Sized;
}
impl<S: Sized> ReadyChunksDynamicExt for S
where
    S: Stream,
{
    fn ready_chunks_dynamic(self) -> ReadyChunksDynamic<Self>
    where
        Self: Sized,
    {
        ReadyChunksDynamic::new(self)
    }
}
