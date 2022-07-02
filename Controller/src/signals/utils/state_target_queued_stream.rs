use super::super::{
    signal::state_target_queued::Signal,
    types::state::Value,
    waker::{TargetsChangedWaker, TargetsChangedWakerStream},
};
use futures::{stream::FusedStream, Stream};
use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
};

pub struct StateTargetQueuedStream<'w, 's, T>
where
    T: Value + Clone,
{
    waker_stream: TargetsChangedWakerStream<'w>,
    signal: &'s Signal<T>,

    buffer: VecDeque<Option<T>>,
}
impl<'w, 's, T> StateTargetQueuedStream<'w, 's, T>
where
    T: Value + Clone,
{
    pub fn new(
        waker: &'w TargetsChangedWaker,
        signal: &'s Signal<T>,
    ) -> Self {
        let waker_stream = waker.stream();
        let buffer = VecDeque::<Option<T>>::new();

        Self {
            waker_stream,
            signal,
            buffer,
        }
    }
}
impl<'w, 's, T> Stream for StateTargetQueuedStream<'w, 's, T>
where
    T: Value + Clone,
{
    type Item = Option<T>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        // poll the inner waker
        let waker_stream_pin = unsafe { Pin::new_unchecked(&mut self_.waker_stream) };
        match waker_stream_pin.poll_next(cx) {
            Poll::Ready(Some(())) => {
                // something was possibly added to the buffer
                // move all items from signal to internal buffer
                let values = self_.signal.take_pending().into_vec().into_iter();
                self_.buffer.extend(values);
            }
            Poll::Ready(None) => {
                // waker_stream should never finish
                panic!("waker_stream yielded");
            }
            Poll::Pending => {
                // no new items available, but we continue
                // because there still could be some items left in the buffer
            }
        }

        // try yielding item if present
        if let Some(value) = self_.buffer.pop_front() {
            Poll::Ready(Some(value))
        } else {
            // nothing in the internal buffer
            Poll::Pending
        }
    }
}
impl<'w, 's, T> FusedStream for StateTargetQueuedStream<'w, 's, T>
where
    T: Value + Clone,
{
    fn is_terminated(&self) -> bool {
        // TargetsChangedWakerStream is never ending
        false
    }
}
