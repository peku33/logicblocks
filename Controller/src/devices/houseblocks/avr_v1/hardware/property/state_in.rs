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
    // pub fn get(&self) -> Option<T> {
    //     self.receiver.borrow().clone()
    // }
    pub fn user_get_stream(&self) -> ValueStream<T> {
        ValueStream::new(self)
    }

    // Device
    pub fn device_is_set(&self) -> bool {
        self.receiver.borrow().is_some()
    }
    pub fn device_set(
        &self,
        value: Option<T>,
    ) {
        let _ = self.sender.broadcast(value);
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
