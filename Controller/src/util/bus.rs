use crossbeam::queue::SegQueue;
use futures::{
    stream::{FusedStream, Stream},
    task::AtomicWaker,
};
use parking_lot::RwLock;
use std::{
    collections::HashSet,
    fmt,
    marker::PhantomPinned,
    pin::Pin,
    ptr::NonNull,
    sync::Arc,
    task::{Context, Poll},
};

struct Common<T>
where
    T: Clone,
{
    receivers: HashSet<NonNull<ReceiverInner<T>>>,
}
impl<T> Common<T>
where
    T: Clone,
{
    fn new() -> Self {
        Self {
            receivers: HashSet::new(),
        }
    }
}

#[derive(Clone)]
pub struct Sender<T>
where
    T: Clone,
{
    common: Arc<RwLock<Common<T>>>,
}
impl<T> Sender<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self::new_from_common(Arc::new(RwLock::new(Common::new())))
    }
    fn new_from_common(common: Arc<RwLock<Common<T>>>) -> Self {
        Self { common }
    }

    pub fn send(
        &self,
        value: T,
    ) {
        self.common.read().receivers.iter().for_each(|receiver| {
            let receiver = unsafe { receiver.as_ref() };
            receiver.queue.push(value.clone());
            receiver.waker.wake();
        });
    }

    pub fn sender(&self) -> Sender<T> {
        Sender::new_from_common(self.common.clone())
    }
    pub fn receiver_factory(&self) -> ReceiverFactory<T> {
        ReceiverFactory::new_from_common(self.common.clone())
    }
    pub fn receiver(&self) -> Receiver<T> {
        Receiver::new_from_common(self.common.clone())
    }
}
impl<T> fmt::Debug for Sender<T>
where
    T: Clone,
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        f.pad("Sender { ... }")
    }
}

#[derive(Clone)]
pub struct ReceiverFactory<T>
where
    T: Clone,
{
    common: Arc<RwLock<Common<T>>>,
}
impl<T> ReceiverFactory<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self::new_from_common(Arc::new(RwLock::new(Common::new())))
    }

    fn new_from_common(common: Arc<RwLock<Common<T>>>) -> Self {
        Self { common }
    }

    pub fn sender(&self) -> Sender<T> {
        Sender::new_from_common(self.common.clone())
    }
    pub fn receiver_factory(&self) -> ReceiverFactory<T> {
        ReceiverFactory::new_from_common(self.common.clone())
    }
    pub fn receiver(&self) -> Receiver<T> {
        Receiver::new_from_common(self.common.clone())
    }
}
impl<T> fmt::Debug for ReceiverFactory<T>
where
    T: Clone,
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        f.pad("ReceiverFactory { ... }")
    }
}

struct ReceiverInner<T>
where
    T: Clone,
{
    queue: SegQueue<T>,
    waker: AtomicWaker,
    pin: PhantomPinned,
}

pub struct Receiver<T>
where
    T: Clone,
{
    common: Arc<RwLock<Common<T>>>,
    receiver_inner: Box<ReceiverInner<T>>,
}
impl<T> Receiver<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self::new_from_common(Arc::new(RwLock::new(Common::new())))
    }

    fn new_from_common(common: Arc<RwLock<Common<T>>>) -> Self {
        let mut receiver_inner = Box::new(ReceiverInner {
            queue: SegQueue::new(),
            waker: AtomicWaker::new(),
            pin: PhantomPinned,
        });
        let receiver_inner_pointer = NonNull::new(&mut *receiver_inner).unwrap();
        common.write().receivers.insert(receiver_inner_pointer);
        Self {
            common,
            receiver_inner,
        }
    }

    pub fn sender(&self) -> Sender<T> {
        Sender::new_from_common(self.common.clone())
    }
    pub fn receiver_factory(&self) -> ReceiverFactory<T> {
        ReceiverFactory::new_from_common(self.common.clone())
    }
    pub fn receiver(&self) -> Receiver<T> {
        Receiver::new_from_common(self.common.clone())
    }
}
impl<T> Stream for Receiver<T>
where
    T: Clone,
{
    type Item = T;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        self.receiver_inner.waker.register(cx.waker());
        match self.receiver_inner.queue.pop() {
            Some(item) => Poll::Ready(Some(item)),
            None => Poll::Pending,
        }
    }
}
impl<T> FusedStream for Receiver<T>
where
    T: Clone,
{
    fn is_terminated(&self) -> bool {
        false
    }
}
impl<T> Drop for Receiver<T>
where
    T: Clone,
{
    fn drop(&mut self) {
        let receiver_inner_pointer = NonNull::new(&mut *self.receiver_inner).unwrap();
        self.common
            .write()
            .receivers
            .remove(&receiver_inner_pointer);
    }
}
impl<T> fmt::Debug for Receiver<T>
where
    T: Clone,
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        f.pad("Receiver { ... }")
    }
}
