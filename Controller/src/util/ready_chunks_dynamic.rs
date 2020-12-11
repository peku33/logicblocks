use futures::stream::{Fuse, FusedStream, Stream, StreamExt};
use std::{
    mem::replace,
    pin::Pin,
    task::{Context, Poll},
};

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
            buffer: Vec::new(),
        }
    }
}
impl<S> Stream for ReadyChunksDynamic<S>
where
    S: Stream,
{
    type Item = Vec<S::Item>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        let mut inner = unsafe { Pin::new_unchecked(&mut self_.inner) };

        loop {
            match inner.as_mut().poll_next(cx) {
                Poll::Pending => {
                    if self_.buffer.is_empty() {
                        return Poll::Pending;
                    } else {
                        return Poll::Ready(Some(replace(&mut self_.buffer, Vec::new())));
                    }
                }
                Poll::Ready(Some(item)) => {
                    self_.buffer.push(item);
                }
                Poll::Ready(None) => {
                    if self_.buffer.is_empty() {
                        return Poll::Ready(None);
                    } else {
                        return Poll::Ready(Some(replace(&mut self_.buffer, Vec::new())));
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
        Self: Sized,
    {
        ReadyChunksDynamic::new(self)
    }
}
impl<S: Sized> ReadyChunksDynamicExt for S where S: Stream {}
