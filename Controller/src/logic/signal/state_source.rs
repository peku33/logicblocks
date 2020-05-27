use super::{SignalBase, SignalRemoteBase, StateValue, ValueAny};
use futures::{
    stream::{BoxStream, Stream, StreamExt},
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
    value: Mutex<Arc<V>>,
    version: AtomicUsize,
    waker: AtomicWaker,
}
impl<V: StateValue + PartialEq + Eq> Inner<V> {
    pub fn new(initial: Arc<V>) -> Self {
        log::trace!("Inner - new called");

        Self {
            value: Mutex::new(initial),
            version: AtomicUsize::new(0),
            waker: AtomicWaker::new(),
        }
    }

    pub fn get(&self) -> Arc<V> {
        log::trace!("Inner - get called");

        self.value.lock().unwrap().clone()
    }
    pub fn set(
        &self,
        value: Arc<V>,
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
    pub fn new(initial: Arc<V>) -> Self {
        log::trace!("Signal - new called");

        let inner = Inner::new(initial);
        Self {
            inner: Arc::new(inner),
        }
    }
    pub fn get(&self) -> Arc<V> {
        log::trace!("Signal - get called");

        self.inner.get()
    }
    pub fn set(
        &self,
        value: Arc<V>,
    ) {
        log::trace!("Signal - set called");

        self.inner.set(value)
    }
}
impl<V: StateValue + PartialEq + Eq> SignalBase for Signal<V> {
    fn remote(&self) -> SignalRemoteBase {
        log::trace!("Signal - remote called");

        SignalRemoteBase::StateSource(Box::new(Remote::new(self.inner.clone())))
    }
}

pub trait RemoteBase: Send + Sync + fmt::Debug {
    fn type_id(&self) -> TypeId;
    fn get(&self) -> Arc<dyn ValueAny>;
    fn get_stream(&self) -> BoxStream<Arc<dyn ValueAny>>;
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
        log::trace!("Remote - new called");

        TypeId::of::<V>()
    }
    fn get(&self) -> Arc<dyn ValueAny> {
        log::trace!("Remote - get called");

        self.inner.get()
    }
    fn get_stream(&self) -> BoxStream<Arc<dyn ValueAny>> {
        log::trace!("Remote - get_stream called");

        RemoteStream::new(self.inner.clone()).boxed()
    }
}

pub struct RemoteStream<V: StateValue + PartialEq + Eq> {
    inner: Arc<Inner<V>>,
    version: AtomicUsize,
}
impl<V: StateValue + PartialEq + Eq> RemoteStream<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("RemoteStream - new called");

        let version = inner.version.load(Ordering::Relaxed);
        Self {
            inner,
            version: AtomicUsize::new(version),
        }
    }
}
impl<V: StateValue + PartialEq + Eq> Stream for RemoteStream<V> {
    type Item = Arc<dyn ValueAny>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        log::trace!("RemoteStream - poll_next called");

        let self_ = unsafe { self.get_unchecked_mut() };

        self_.inner.waker.register(cx.waker());

        let version = self_.inner.version.load(Ordering::SeqCst);
        if self_.version.swap(version, Ordering::Relaxed) != version {
            return Poll::Ready(Some(self_.inner.get()));
        }
        Poll::Pending
    }
}
