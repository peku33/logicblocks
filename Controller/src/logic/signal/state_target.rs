use super::{SignalBase, SignalRemoteBase, StateValue, ValueAny};
use futures::{
    stream::{FusedStream, Stream},
    task::AtomicWaker,
};
use std::{
    any::TypeId,
    fmt,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
};

#[derive(Debug)]
struct Inner<V: StateValue + PartialEq + Eq> {
    value: Mutex<Option<Arc<V>>>,
    version: AtomicUsize,
    waker: AtomicWaker,
}
impl<V: StateValue + PartialEq + Eq> Inner<V> {
    pub fn new() -> Self {
        log::trace!("Inner - new called");

        Self {
            value: Mutex::new(None),
            version: AtomicUsize::new(0),
            waker: AtomicWaker::new(),
        }
    }

    pub fn get(&self) -> Option<Arc<V>> {
        log::trace!("Inner - get called");

        self.value.lock().unwrap().clone()
    }
    pub fn set(
        &self,
        value: Option<Arc<V>>,
    ) {
        log::trace!("Inner - set called");

        let mut previous_value = self.value.lock().unwrap();
        if *previous_value == value {
            log::trace!("set - value is the same, skipping");

            return;
        }

        log::trace!(
            "set - value is different, changing {:?} -> {:?}",
            *previous_value,
            value
        );

        *previous_value = value;
        self.version.fetch_add(1, Ordering::SeqCst);
        self.waker.wake();
    }
}
impl<V: StateValue + PartialEq + Eq> Drop for Inner<V> {
    fn drop(&mut self) {
        log::trace!("Inner - drop called");
    }
}

pub struct Signal<V: StateValue + PartialEq + Eq> {
    inner: Arc<Inner<V>>,
}
impl<V: StateValue + PartialEq + Eq> Signal<V> {
    pub fn new() -> Self {
        log::trace!("Signal - new called");

        Self {
            inner: Arc::new(Inner::new()),
        }
    }
    pub fn get(&self) -> Option<Arc<V>> {
        log::trace!("Signal - get called");

        self.inner.get()
    }
    pub fn get_stream(&self) -> ValueStream<V> {
        log::trace!("Signal - get_stream called");

        ValueStream::new(self.inner.clone())
    }
}
impl<V: StateValue + PartialEq + Eq> SignalBase for Signal<V> {
    fn remote(&self) -> SignalRemoteBase {
        log::trace!("Signal - remote called");

        SignalRemoteBase::StateTarget(Box::new(Remote::new(self.inner.clone())))
    }
}

pub struct ValueStream<V: StateValue + PartialEq + Eq> {
    inner: Arc<Inner<V>>,
    version: AtomicUsize,
}
impl<V: StateValue + PartialEq + Eq> ValueStream<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("ValueStream - new called");

        let version = inner.version.load(Ordering::Relaxed);
        Self {
            inner,
            version: AtomicUsize::new(version),
        }
    }
}
impl<V: StateValue + PartialEq + Eq> Stream for ValueStream<V> {
    type Item = Option<Arc<V>>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        log::trace!("ValueStream - poll_next called");

        let self_ = unsafe { self.get_unchecked_mut() };

        self_.inner.waker.register(cx.waker());

        let version = self_.inner.version.load(Ordering::SeqCst);
        if self_.version.swap(version, Ordering::Relaxed) != version {
            return Poll::Ready(Some(self_.inner.get()));
        }
        Poll::Pending
    }
}
impl<V: StateValue + PartialEq + Eq> FusedStream for ValueStream<V> {
    fn is_terminated(&self) -> bool {
        log::trace!("ValueStream - is_terminated called");

        false
    }
}

pub trait RemoteBase: Send + Sync + fmt::Debug {
    fn type_id(&self) -> TypeId;
    fn set_none(&self);
    fn set_unwrap(
        &self,
        value: Arc<dyn ValueAny>,
    );
}
#[derive(Debug)]
pub struct Remote<V: StateValue + PartialEq + Eq> {
    inner: Arc<Inner<V>>,
}
impl<V: StateValue + PartialEq + Eq> Remote<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("Remote - new called");

        Self { inner }
    }
}
impl<V: StateValue + PartialEq + Eq> RemoteBase for Remote<V> {
    fn type_id(&self) -> TypeId {
        log::trace!("Remote - type_id called");

        TypeId::of::<V>()
    }
    fn set_none(&self) {
        log::trace!("Remote - set_none called");

        self.inner.set(None)
    }
    fn set_unwrap(
        &self,
        value: Arc<dyn ValueAny>,
    ) {
        log::trace!("Remote - set_unwrap called");

        let value = match value.downcast::<V>() {
            Ok(value) => value,
            Err(value) => panic!(
                "set_unwrap mismatched type (got: {:?}, expects: {:?})",
                value.type_id(),
                TypeId::of::<V>()
            ),
        };
        self.inner.set(Some(value));
    }
}
