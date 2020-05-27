use super::{EventValue, SignalBase, SignalRemoteBase, ValueAny};
use crossbeam::queue::SegQueue;
use futures::{
    stream::{BoxStream, FusedStream, Stream, StreamExt},
    task::AtomicWaker,
};
use std::{
    any::TypeId,
    fmt,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

#[derive(Debug)]
struct Inner<V: EventValue> {
    queue: SegQueue<Arc<V>>,
    waker: AtomicWaker,
    remote_borrowed: AtomicBool,
    remote_stream_borrowed: AtomicBool,
}
impl<V: EventValue> Inner<V> {
    pub fn new() -> Self {
        log::trace!("Inner - new called");

        Self {
            queue: SegQueue::new(),
            waker: AtomicWaker::new(),
            remote_borrowed: AtomicBool::new(false),
            remote_stream_borrowed: AtomicBool::new(false),
        }
    }
    pub fn pop(&self) -> Option<Arc<V>> {
        log::trace!("Inner - pop called");

        self.queue.pop().ok()
    }
    pub fn push(
        &self,
        value: Arc<V>,
    ) {
        log::trace!("Inner - push called");

        self.queue.push(value);
        self.waker.wake();
    }
}
impl<V: EventValue> Drop for Inner<V> {
    fn drop(&mut self) {
        log::trace!("Inner - drop called");
    }
}

pub struct Signal<V: EventValue> {
    inner: Arc<Inner<V>>,
}
impl<V: EventValue> Signal<V> {
    pub fn new() -> Self {
        log::trace!("Signal - new called");

        Self {
            inner: Arc::new(Inner::new()),
        }
    }
    pub fn push(
        &self,
        value: Arc<V>,
    ) {
        log::trace!("Signal - push called");

        self.inner.push(value);
    }
}
impl<V: EventValue> SignalBase for Signal<V> {
    fn remote(&self) -> SignalRemoteBase {
        log::trace!("Signal - remote called");

        SignalRemoteBase::EventSource(Box::new(Remote::new(self.inner.clone())))
    }
}

pub trait RemoteBase: Send + Sync + fmt::Debug {
    fn type_id(&self) -> TypeId;
    fn get_stream(&self) -> BoxStream<Arc<dyn ValueAny>>;
}
#[derive(Debug)]
pub struct Remote<V: EventValue> {
    inner: Arc<Inner<V>>,
}
impl<V: EventValue> Remote<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("Remote - new called");

        if inner.remote_borrowed.swap(true, Ordering::Relaxed) {
            panic!("remote already borrowed");
        }
        Self { inner }
    }
}
impl<V: EventValue> RemoteBase for Remote<V> {
    fn type_id(&self) -> TypeId {
        log::trace!("Remote - type_id called");

        TypeId::of::<V>()
    }
    fn get_stream(&self) -> BoxStream<Arc<dyn ValueAny>> {
        log::trace!("Remote - get_stream called");

        RemoteStream::new(self.inner.clone()).boxed()
    }
}
impl<V: EventValue> Drop for Remote<V> {
    fn drop(&mut self) {
        log::trace!("Remote - drop called");

        if !self.inner.remote_borrowed.swap(false, Ordering::Relaxed) {
            panic!("remote not borrowed");
        }
    }
}

pub struct RemoteStream<V: EventValue> {
    inner: Arc<Inner<V>>,
}
impl<V: EventValue> RemoteStream<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("RemoteStream - new called");

        if inner.remote_stream_borrowed.swap(true, Ordering::Relaxed) {
            panic!("remote already borrowed");
        }
        Self { inner }
    }
}
impl<V: EventValue> Stream for RemoteStream<V> {
    type Item = Arc<dyn ValueAny>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        log::trace!("RemoteStream - poll_next called");

        let self_ = unsafe { self.get_unchecked_mut() };

        self_.inner.waker.register(cx.waker());
        match self_.inner.pop() {
            Some(value) => Poll::Ready(Some(value)),
            None => Poll::Pending,
        }
    }
}
impl<V: EventValue> FusedStream for RemoteStream<V> {
    fn is_terminated(&self) -> bool {
        log::trace!("RemoteStream - is_terminated called");

        false
    }
}
impl<V: EventValue> Drop for RemoteStream<V> {
    fn drop(&mut self) {
        log::trace!("RemoteStream - drop called");

        if !self
            .inner
            .remote_stream_borrowed
            .swap(false, Ordering::Relaxed)
        {
            panic!("remote not borrowed");
        }
    }
}
