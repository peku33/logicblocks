use super::super::{
    signal::state_target_queued::Signal,
    types::state::Value,
    waker::{TargetsChangedWaker, TargetsChangedWakerStream},
};
use futures::{Stream, stream::FusedStream};
use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
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
impl<T> Stream for StateTargetQueuedStream<'_, '_, T>
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
        match unsafe { Pin::new_unchecked(&mut self_.waker_stream) }.poll_next(cx) {
            Poll::Ready(Some(())) => {
                // something was possibly added to the buffer
                // move all items from signal to internal buffer
                let values = self_.signal.take_pending().into_iter();
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
impl<T> FusedStream for StateTargetQueuedStream<'_, '_, T>
where
    T: Value + Clone,
{
    fn is_terminated(&self) -> bool {
        // TargetsChangedWakerStream is never ending
        false
    }
}
