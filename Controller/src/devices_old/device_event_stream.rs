use crate::util::bus2;
use futures::pin_mut;
use futures::stream::Stream;
use futures::task::{Context, Poll};
use serde::Serialize;
use std::borrow::Cow;
use std::pin::Pin;

pub type Item = Cow<'static, str>;

pub struct Sender {
    inner: bus2::Sender<Item>,
}
impl Sender {
    fn new(inner: bus2::Sender<Item>) -> Self {
        Self { inner }
    }
    pub fn send_str(
        &self,
        item: &'static str,
    ) {
        self.inner.send(Cow::from(item))
    }
    pub fn send_string(
        &self,
        item: String,
    ) {
        self.inner.send(Cow::from(item))
    }
    pub fn send_empty(&self) {
        self.send_str("")
    }
    pub fn send_json<T: Serialize>(
        &self,
        item: &T,
    ) {
        self.send_string(serde_json::to_string(item).unwrap())
    }
}

pub struct ReceiverFactory {
    inner: bus2::ReceiverFactory<Item>,
}
impl ReceiverFactory {
    fn new(inner: bus2::ReceiverFactory<Item>) -> Self {
        Self { inner }
    }
    pub fn receiver(&self) -> Receiver {
        Receiver::new(self.inner.receiver())
    }
}

pub struct Receiver {
    inner: bus2::Receiver<Item>,
}
impl Receiver {
    fn new(inner: bus2::Receiver<Item>) -> Self {
        Self { inner }
    }
}
impl Stream for Receiver {
    type Item = Item;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let self_ = self.get_mut();
        let inner = &mut self_.inner;
        pin_mut!(inner);
        inner.poll_next(cx)
    }
}

pub fn channel() -> (Sender, ReceiverFactory) {
    let (sender, receiver_factory) = bus2::channel();
    let sender = Sender::new(sender);
    let receiver_factory = ReceiverFactory::new(receiver_factory);
    (sender, receiver_factory)
}
