use super::{SignalBase, SignalRemoteBase, StateValue, ValueAny};
use futures::{
    sink::Sink,
    stream::{BoxStream, Stream, StreamExt},
    task::AtomicWaker,
};
use parking_lot::Mutex;
use std::{
    any::{type_name, TypeId},
    convert::Infallible,
    fmt,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

#[derive(Debug)]
struct Inner<V: StateValue + Clone + PartialEq> {
    value: Mutex<V>,
    version: AtomicUsize,
    waker: AtomicWaker,
}
impl<V: StateValue + Clone + PartialEq> Inner<V> {
    pub fn new(initial: V) -> Self {
        log::trace!("Inner - new called");

        Self {
            value: Mutex::new(initial),
            version: AtomicUsize::new(0),
            waker: AtomicWaker::new(),
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
    pub fn new(initial: V) -> Self {
        log::trace!("Signal - new called");

        let inner = Inner::new(initial);
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn current(&self) -> V {
        log::trace!("Signal - current called");

        self.inner.value.lock().clone()
    }

    pub fn set(
        &self,
        value: V,
    ) {
        log::trace!("Signal - set called");

        let mut current_value = self.inner.value.lock();
        if *current_value != value {
            log::trace!("Signal - set value changed");

            *current_value = value;
            self.inner.version.fetch_add(1, Ordering::Relaxed);
        }
        drop(current_value);

        self.inner.waker.wake();
    }

    pub fn sink(&self) -> ValueSink<V> {
        log::trace!("Signal - sink called");

        ValueSink::new(self)
    }
}
impl<V: StateValue + Clone + PartialEq> SignalBase for Signal<V> {
    fn remote(&self) -> SignalRemoteBase {
        log::trace!("Signal - remote called");

        SignalRemoteBase::StateSource(Box::new(Remote::new(self.inner.clone())))
    }
}

pub struct ValueSink<'s, V: StateValue + Clone + PartialEq> {
    signal: &'s Signal<V>,
}
impl<'s, V: StateValue + Clone + PartialEq> ValueSink<'s, V> {
    fn new(signal: &'s Signal<V>) -> Self {
        log::trace!("ValueSink - new called");

        Self { signal }
    }
}
impl<'s, V: StateValue + Clone + PartialEq> Sink<V> for ValueSink<'s, V> {
    type Error = Infallible;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        log::trace!("ValueSink - poll_ready called");

        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: Pin<&mut Self>,
        value: V,
    ) -> Result<(), Self::Error> {
        log::trace!("ValueSink - start_send called");

        let mut current_value = self.signal.inner.value.lock();
        if *current_value != value {
            log::trace!("ValueSink - start_send value changed");

            *current_value = value;
            self.signal.inner.version.fetch_add(1, Ordering::Relaxed);
        }
        drop(current_value);

        Ok(())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        log::trace!("ValueSink - poll_flush called");

        self.signal.inner.waker.wake();

        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        log::trace!("ValueSink - poll_close called");

        Poll::Ready(Ok(()))
    }
}

pub trait RemoteBase: Send + Sync + fmt::Debug {
    fn type_id(&self) -> TypeId;
    fn type_name(&self) -> &'static str;
    fn current(&self) -> Box<dyn ValueAny>;
    fn stream(&self) -> BoxStream<Box<dyn ValueAny>>;
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
        log::trace!("Remote - new called");

        TypeId::of::<V>()
    }
    fn type_name(&self) -> &'static str {
        log::trace!("Remote - type_name called");

        type_name::<V>()
    }
    fn current(&self) -> Box<dyn ValueAny> {
        log::trace!("Remote - current called");

        let value = self.inner.value.lock().clone();
        Box::new(value)
    }
    fn stream(&self) -> BoxStream<Box<dyn ValueAny>> {
        log::trace!("Remote - stream called");

        RemoteStream::new(self.inner.clone()).boxed()
    }
}

pub struct RemoteStream<V: StateValue + Clone + PartialEq> {
    inner: Arc<Inner<V>>,
    version: AtomicUsize,
}
impl<V: StateValue + Clone + PartialEq> RemoteStream<V> {
    fn new(inner: Arc<Inner<V>>) -> Self {
        log::trace!("RemoteStream - new called");

        let version = inner.version.load(Ordering::Relaxed);
        Self {
            inner,
            version: AtomicUsize::new(version),
        }
    }
}
impl<V: StateValue + Clone + PartialEq> Stream for RemoteStream<V> {
    type Item = Box<dyn ValueAny>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        log::trace!("RemoteStream - poll_next called");

        self.inner.waker.register(cx.waker());

        let version = self.inner.version.load(Ordering::SeqCst);
        if self.version.swap(version, Ordering::Relaxed) == version {
            return Poll::Pending;
        }

        let value = self.inner.value.lock().clone();
        let value = Box::new(value);
        Poll::Ready(Some(value))
    }
}
