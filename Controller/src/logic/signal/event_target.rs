use super::{EventValue, SignalBase, SignalRemoteBase, ValueAny};
use crossbeam::queue::SegQueue;
use futures::{
    stream::{FusedStream, Stream},
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
    stream_borrowed: AtomicBool,
}
impl<V: EventValue> Inner<V> {
    pub fn new() -> Self {
        log::trace!("Inner - new called");

        Self {
            queue: SegQueue::new(),
            waker: AtomicWaker::new(),
            stream_borrowed: AtomicBool::new(false),
        }
    }
    pub fn pop(&self) -> Option<Arc<V>> {
        log::trace!("Inner - pop");

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
    pub fn get_stream(&self) -> ValueStream<V> {
        log::trace!("Signal - get_stream called");

        ValueStream::new(self.inner.clone())
    }
}
impl<V: EventValue> SignalBase for Signal<V> {
    fn remote(&self) -> SignalRemoteBase {
        log::trace!("Signal - remote called");

        SignalRemoteBase::EventTarget(Box::new(Remote::new(self.inner.clone())))
    }
}

pub struct ValueStream<V: EventValue> {
    inner: Arc<Inner<V>>,
}
impl<V: EventValue> ValueStream<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("ValueStream - new called");

        if inner.stream_borrowed.swap(true, Ordering::Relaxed) {
            panic!("stream already borrowed");
        }
        Self { inner }
    }
}
impl<V: EventValue> Stream for ValueStream<V> {
    type Item = Arc<V>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        log::trace!("ValueStream - poll_next called");

        let self_ = unsafe { self.get_unchecked_mut() };

        self_.inner.waker.register(cx.waker());
        match self_.inner.pop() {
            Some(value) => Poll::Ready(Some(value)),
            None => Poll::Pending,
        }
    }
}
impl<V: EventValue> FusedStream for ValueStream<V> {
    fn is_terminated(&self) -> bool {
        log::trace!("ValueStream - is_terminated called");

        false
    }
}
impl<V: EventValue> Drop for ValueStream<V> {
    fn drop(&mut self) {
        log::trace!("ValueStream - drop called");

        if !self.inner.stream_borrowed.swap(false, Ordering::Relaxed) {
            panic!("stream not borrowed");
        }
    }
}

pub trait RemoteBase: Send + Sync + fmt::Debug {
    fn type_id(&self) -> TypeId;
    fn push_unwrap(
        &self,
        value: Arc<dyn ValueAny>,
    );
}
#[derive(Debug)]
pub struct Remote<V: EventValue> {
    inner: Arc<Inner<V>>,
}
impl<V: EventValue> Remote<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("Remote - new called");

        Self { inner }
    }
}
impl<V: EventValue> RemoteBase for Remote<V> {
    fn type_id(&self) -> TypeId {
        log::trace!("Remote - type_id called");

        TypeId::of::<V>()
    }
    fn push_unwrap(
        &self,
        value: Arc<dyn ValueAny>,
    ) {
        log::trace!("Remote - push_unwrap called");

        let value = match value.downcast::<V>() {
            Ok(value) => value,
            Err(value) => panic!(
                "push_unwrap mismatched type (got: {:?}, expects: {:?})",
                value.type_id(),
                TypeId::of::<V>()
            ),
        };
        self.inner.push(value);
    }
}
