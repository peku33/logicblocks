use super::{SignalBase, SignalRemoteBase, StateValue, ValueAny};
use futures::{
    stream::{FusedStream, Stream},
    task::AtomicWaker,
};
use parking_lot::Mutex;
use std::{
    any::{type_name, TypeId},
    fmt,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

#[derive(Debug)]
struct Inner<V: StateValue + Clone + PartialEq> {
    value: Mutex<Option<V>>,
    version: AtomicUsize,
    waker: AtomicWaker,
    stream_borrowed: AtomicBool,
}
impl<V: StateValue + Clone + PartialEq> Inner<V> {
    pub fn new() -> Self {
        log::trace!("Inner - new called");

        Self {
            value: Mutex::new(None),
            version: AtomicUsize::new(1), // Streams starts with zero to get value immediately
            waker: AtomicWaker::new(),
            stream_borrowed: AtomicBool::new(false),
        }
    }
}
impl<V: StateValue + Clone + PartialEq> Drop for Inner<V> {
    fn drop(&mut self) {
        log::trace!("Inner - drop called");
    }
}

pub struct Signal<V: StateValue + Clone + PartialEq> {
    inner: Arc<Inner<V>>,
}
impl<V: StateValue + Clone + PartialEq> Signal<V> {
    pub fn new() -> Self {
        log::trace!("Signal - new called");

        Self {
            inner: Arc::new(Inner::new()),
        }
    }

    pub fn current(&self) -> Option<V> {
        log::trace!("Signal - get called");

        self.inner.value.lock().clone()
    }
    pub fn stream(&self) -> ValueStream<V> {
        log::trace!("Signal - stream called");

        ValueStream::new(self)
    }
}
impl<V: StateValue + Clone + PartialEq> SignalBase for Signal<V> {
    fn remote(&self) -> SignalRemoteBase {
        log::trace!("Signal - remote called");

        SignalRemoteBase::StateTarget(Box::new(Remote::new(self.inner.clone())))
    }
}

pub struct ValueStream<'s, V: StateValue + Clone + PartialEq> {
    signal: &'s Signal<V>,
    version: AtomicUsize,
}
impl<'s, V: StateValue + Clone + PartialEq> ValueStream<'s, V> {
    fn new(signal: &'s Signal<V>) -> Self {
        log::trace!("ValueStream - new called");

        if signal.inner.stream_borrowed.swap(true, Ordering::Relaxed) {
            panic!("stream already borrowed");
        }

        Self {
            signal,
            version: AtomicUsize::new(0),
        }
    }
}
impl<'s, V: StateValue + Clone + PartialEq> Stream for ValueStream<'s, V> {
    type Item = Option<V>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        log::trace!("ValueStream - poll_next called");

        let self_ = unsafe { self.get_unchecked_mut() };

        self_.signal.inner.waker.register(cx.waker());

        let version = self_.signal.inner.version.load(Ordering::SeqCst);
        if self_.version.swap(version, Ordering::Relaxed) == version {
            return Poll::Pending;
        }

        let value = self_.signal.inner.value.lock().clone();
        Poll::Ready(Some(value))
    }
}
impl<'s, V: StateValue + Clone + PartialEq> FusedStream for ValueStream<'s, V> {
    fn is_terminated(&self) -> bool {
        log::trace!("ValueStream - is_terminated called");

        false
    }
}
impl<'s, V: StateValue + Clone + PartialEq> Drop for ValueStream<'s, V> {
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
    fn set(
        &self,
        value: Option<&dyn ValueAny>,
    );
}
#[derive(Debug)]
pub struct Remote<V: StateValue + Clone + PartialEq> {
    inner: Arc<Inner<V>>,
}
impl<V: StateValue + Clone + PartialEq> Remote<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("Remote - new called");

        Self { inner }
    }
}
impl<V: StateValue + Clone + PartialEq> RemoteBase for Remote<V> {
    fn type_id(&self) -> TypeId {
        log::trace!("Remote - type_id called");

        TypeId::of::<V>()
    }
    fn type_name(&self) -> &'static str {
        log::trace!("Remote - type_name called");

        type_name::<V>()
    }
    fn set(
        &self,
        value: Option<&dyn ValueAny>,
    ) {
        log::trace!("Remote - set called");

        let value = value.map(|value| match value.downcast_ref::<V>() {
            Some(value) => value,
            None => panic!(
                "set mismatched type (got: {:?}, expects: {:?})",
                value.type_id(),
                TypeId::of::<V>()
            ),
        });

        let mut current_value = self.inner.value.lock();
        if current_value.as_ref() != value {
            log::trace!("Remote - set value changed");

            *current_value = value.cloned();
            self.inner.version.fetch_add(1, Ordering::Relaxed);
        }
        drop(current_value);

        self.inner.waker.wake();
    }
}
