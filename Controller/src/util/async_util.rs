use futures::{
    stream::{FusedStream, Stream},
    task::{Context, Poll},
};
use std::pin::Pin;

pub struct InfiniteStream<S>
where
    S: Stream,
{
    stream: S,
    completed: bool,
}
impl<S> InfiniteStream<S>
where
    S: Stream,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            completed: false,
        }
    }
}
impl<S> Stream for InfiniteStream<S>
where
    S: Stream,
{
    type Item = S::Item;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let mut self_ = unsafe { self.get_unchecked_mut() };
        if self_.completed {
            return Poll::Pending;
        }

        let stream = unsafe { Pin::new_unchecked(&mut self_.stream) };
        match stream.poll_next(cx) {
            Poll::Ready(Some(value)) => Poll::Ready(Some(value)),
            Poll::Ready(None) => {
                self_.completed = true;
                Poll::Pending
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
impl<S> FusedStream for InfiniteStream<S>
where
    S: Stream,
{
    fn is_terminated(&self) -> bool {
        false
    }
}
