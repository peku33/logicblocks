use futures::stream::{FusedStream, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::watch;

#[derive(Debug)]
pub struct Property<T>
where
    T: Clone + PartialEq,
{
    sender: watch::Sender<Option<T>>,
    receiver: watch::Receiver<Option<T>>,
}
impl<T> Property<T>
where
    T: Clone + PartialEq,
{
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(None);
        Self { sender, receiver }
    }

    // User
    pub fn user_get_stream(&self) -> ValueStream<T> {
        ValueStream::new(self)
    }

    // Device
    pub fn device_is_set(&self) -> bool {
        self.receiver.borrow().is_some()
    }
    pub fn device_set(
        &self,
        value: T,
    ) {
        let _ = self.sender.broadcast(Some(value));
    }
    pub fn device_set_unknown(&self) {
        let _ = self.sender.broadcast(None);
    }
}

pub struct ValueStream<'p, T>
where
    T: Clone + PartialEq,
{
    parent: &'p Property<T>,
    receiver: watch::Receiver<Option<T>>,
    completed: bool,
}
impl<'p, T> ValueStream<'p, T>
where
    T: Clone + PartialEq,
{
    fn new(parent: &'p Property<T>) -> Self {
        Self {
            parent,
            receiver: parent.receiver.clone(),
            completed: false,
        }
    }
}

impl<'p, T> Stream for ValueStream<'p, T>
where
    T: Clone + PartialEq,
{
    type Item = Option<T>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        if self_.completed {
            return Poll::Pending;
        }

        let receiver = unsafe { Pin::new_unchecked(&mut self_.receiver) };
        match receiver.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                self_.completed = true;
                Poll::Pending
            }
            Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
        }
    }
}
impl<'p, T> FusedStream for ValueStream<'p, T>
where
    T: Clone + PartialEq,
{
    fn is_terminated(&self) -> bool {
        false
    }
}
