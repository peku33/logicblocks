use futures::stream::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

// FutureOrPending is same as future::Fuse

#[derive(Debug)]
pub struct StreamOrPending<S: Stream> {
    inner: Option<S>,
}
// impl<S: Stream + Unpin> Unpin for StreamOrPending<S> {}
impl<S: Stream> StreamOrPending<S> {
    pub fn new(inner: Option<S>) -> Self {
        Self { inner }
    }

    pub fn future(inner: S) -> Self {
        Self { inner: Some(inner) }
    }
    pub fn pending() -> Self {
        Self { inner: None }
    }
}
impl<S: Stream> Stream for StreamOrPending<S> {
    type Item = S::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        match self_.inner.as_mut() {
            Some(inner) => unsafe { Pin::new_unchecked(inner).poll_next(cx) },
            None => Poll::Pending,
        }
    }
}
