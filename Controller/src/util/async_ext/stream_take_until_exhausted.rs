use futures::{
    future::Future,
    stream::{FusedStream, Stream},
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub struct StreamTakeUntilExhausted<S, F>
where
    S: Stream,
    F: Future<Output = ()>,
{
    stream: Option<S>,
    take_until: Option<F>,
}
impl<S, F> StreamTakeUntilExhausted<S, F>
where
    S: Stream,
    F: Future<Output = ()>,
{
    pub fn new(
        stream: S,
        take_until: F,
    ) -> Self {
        Self {
            stream: Some(stream),
            take_until: Some(take_until),
        }
    }
}
impl<S, F> Stream for StreamTakeUntilExhausted<S, F>
where
    S: Stream,
    F: Future<Output = ()>,
{
    type Item = S::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        let stream = &mut self_.stream;
        let take_until = &mut self_.take_until;

        // if the canceling future completed - free the future
        if let Some(take_until_some) = take_until {
            let take_until_some_pin = unsafe { Pin::new_unchecked(take_until_some) };
            if let Poll::Ready(()) = take_until_some_pin.poll(cx) {
                take_until.take();
            }
        }

        let mut stream_item = Option::<S::Item>::None;
        if let Some(stream_some) = stream {
            let stream_some_pin = unsafe { Pin::new_unchecked(stream_some) };
            match stream_some_pin.poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    // keep forwarding items
                    stream_item.replace(item);
                }
                Poll::Ready(None) => {
                    stream.take();
                }
                Poll::Pending => {
                    // if the canceling future yielded, we also cancel ourselves
                    if take_until.is_none() {
                        stream.take();
                    }
                }
            }
        }

        if let Some(stream_item) = stream_item {
            Poll::Ready(Some(stream_item))
        } else if stream.is_none() && take_until.is_none() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}
impl<S, F> FusedStream for StreamTakeUntilExhausted<S, F>
where
    S: Stream,
    F: Future<Output = ()>,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_none() && self.take_until.is_none()
    }
}

pub trait StreamTakeUntilExhaustedExt: Stream {
    fn stream_take_until_exhausted<F>(
        self,
        take_until: F,
    ) -> StreamTakeUntilExhausted<Self, F>
    where
        Self: Sized,
        F: Future<Output = ()>,
    {
        StreamTakeUntilExhausted::new(self, take_until)
    }
}
impl<S: Sized> StreamTakeUntilExhaustedExt for S where S: Stream {}
