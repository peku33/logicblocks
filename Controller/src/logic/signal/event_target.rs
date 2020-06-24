use super::{EventValue, SignalBase, SignalRemoteBase, ValueAny};
use crossbeam::queue::{PopError, SegQueue};
use futures::{
    stream::{FusedStream, Stream},
    task::AtomicWaker,
};
use std::{
    any::{type_name, TypeId},
    fmt,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

#[derive(Debug)]
struct Inner<V: EventValue + Clone> {
    queue: SegQueue<V>,
    waker: AtomicWaker,
    stream_borrowed: AtomicBool,
}
impl<V: EventValue + Clone> Inner<V> {
    pub fn new() -> Self {
        log::trace!("Inner - new called");

        Self {
            queue: SegQueue::new(),
            waker: AtomicWaker::new(),
            stream_borrowed: AtomicBool::new(false),
        }
    }
}
impl<V: EventValue + Clone> Drop for Inner<V> {
    fn drop(&mut self) {
        log::trace!("Inner - drop called");
    }
}

pub struct Signal<V: EventValue + Clone> {
    inner: Arc<Inner<V>>,
}
impl<V: EventValue + Clone> Signal<V> {
    pub fn new() -> Self {
        log::trace!("Signal - new called");

        Self {
            inner: Arc::new(Inner::new()),
        }
    }

    pub fn stream(&self) -> ValueStream<V> {
        log::trace!("Signal - stream called");

        ValueStream::new(self)
    }
}
impl<V: EventValue + Clone> SignalBase for Signal<V> {
    fn remote(&self) -> SignalRemoteBase {
        log::trace!("Signal - remote called");

        SignalRemoteBase::EventTarget(Box::new(Remote::new(self.inner.clone())))
    }
}

pub struct ValueStream<'s, V: EventValue + Clone> {
    signal: &'s Signal<V>,
}
impl<'s, V: EventValue + Clone> ValueStream<'s, V> {
    fn new(signal: &'s Signal<V>) -> Self {
        log::trace!("ValueStream - new called");

        if signal.inner.stream_borrowed.swap(true, Ordering::Relaxed) {
            panic!("stream already borrowed");
        }
        Self { signal }
    }
}
impl<'s, V: EventValue + Clone> Stream for ValueStream<'s, V> {
    type Item = V;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        log::trace!("ValueStream - poll_next called");

        self.signal.inner.waker.register(cx.waker());
        match self.signal.inner.queue.pop() {
            Ok(value) => Poll::Ready(Some(value)),
            Err(PopError) => Poll::Pending,
        }
    }
}
impl<'s, V: EventValue + Clone> FusedStream for ValueStream<'s, V> {
    fn is_terminated(&self) -> bool {
        log::trace!("ValueStream - is_terminated called");

        false
    }
}
impl<'s, V: EventValue + Clone> Drop for ValueStream<'s, V> {
    fn drop(&mut self) {
        log::trace!("ValueStream - drop called");

        if !self
            .signal
            .inner
            .stream_borrowed
            .swap(false, Ordering::Relaxed)
        {
            panic!("stream not borrowed");
        }
    }
}

pub trait RemoteBase: Send + Sync + fmt::Debug {
    fn type_id(&self) -> TypeId;
    fn type_name(&self) -> &'static str;
    fn push(
        &self,
        value: &dyn ValueAny,
    );
}

#[derive(Debug)]
pub struct Remote<V: EventValue + Clone> {
    inner: Arc<Inner<V>>,
}
impl<V: EventValue + Clone> Remote<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("Remote - new called");

        Self { inner }
    }
}
impl<V: EventValue + Clone> RemoteBase for Remote<V> {
    fn type_id(&self) -> TypeId {
        log::trace!("Remote - type_id called");

        TypeId::of::<V>()
    }
    fn type_name(&self) -> &'static str {
        log::trace!("Remote - type_name called");

        type_name::<V>()
    }
    fn push(
        &self,
        value: &dyn ValueAny,
    ) {
        log::trace!("Remote - push called");

        let value = match value.downcast_ref::<V>() {
            Some(value) => value,
            None => panic!(
                "push mismatched type (got: {:?}, expects: {:?})",
                value.type_id(),
                TypeId::of::<V>()
            ),
        };
        self.inner.queue.push(value.clone());

        self.inner.waker.wake();
    }
}
