use futures::{
    stream::{FusedStream, Stream},
    task::{AtomicWaker, Context, Poll},
};
use std::{
    collections::HashSet,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

#[derive(PartialEq, Eq, Hash, Debug)]
struct ReceiverInnerPointer(*const ReceiverInner);
unsafe impl Sync for ReceiverInnerPointer {}
unsafe impl Send for ReceiverInnerPointer {}

#[derive(Debug)]
struct Common {
    version: AtomicUsize,
    receivers: RwLock<HashSet<ReceiverInnerPointer>>,
}
impl Common {
    pub fn new() -> Self {
        let version = 0;
        let version = AtomicUsize::new(version);

        let receivers = HashSet::<ReceiverInnerPointer>::new();
        let receivers = RwLock::new(receivers);

        Self { version, receivers }
    }
    pub fn wake(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
        self.receivers
            .read()
            .unwrap()
            .iter()
            .for_each(|receiver_inner_pointer| {
                let receiver_common: &ReceiverInner = unsafe { &*receiver_inner_pointer.0 };
                receiver_common.waker.wake();
            });
    }
}

#[derive(Debug)]
pub struct Sender {
    common: Arc<Common>,
}
impl Sender {
    pub fn new() -> Self {
        let common = Common::new();
        Self {
            common: Arc::new(common),
        }
    }
    pub fn wake(&self) {
        self.common.wake();
    }
    pub fn receiver_factory(&self) -> ReceiverFactory {
        ReceiverFactory::new(self.common.clone())
    }
    pub fn receiver(&self) -> Receiver {
        Receiver::new(self.common.clone())
    }
}

#[derive(Debug)]
pub struct ReceiverFactory {
    common: Arc<Common>,
}
impl ReceiverFactory {
    fn new(common: Arc<Common>) -> Self {
        Self { common }
    }
    pub fn receiver(&self) -> Receiver {
        Receiver::new(self.common.clone())
    }
}

#[derive(Debug)]
struct ReceiverInner {
    waker: AtomicWaker,
}

#[derive(Debug)]
pub struct Receiver {
    common: Arc<Common>,
    version: AtomicUsize,
    inner: Box<ReceiverInner>,
}
impl Receiver {
    fn new(common: Arc<Common>) -> Self {
        let version = common.version.load(Ordering::Relaxed);
        let version = AtomicUsize::new(version);

        let inner = ReceiverInner {
            waker: AtomicWaker::new(),
        };
        let inner = Box::new(inner);

        common
            .receivers
            .write()
            .unwrap()
            .insert(ReceiverInnerPointer(&*inner));

        Self {
            common,
            version,
            inner,
        }
    }
}
impl Stream for Receiver {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        self.inner.waker.register(cx.waker());

        let version = self.common.version.load(Ordering::SeqCst);
        if self.version.swap(version, Ordering::SeqCst) != version {
            Poll::Ready(Some(()))
        } else {
            Poll::Pending
        }
    }
}
impl FusedStream for Receiver {
    fn is_terminated(&self) -> bool {
        false
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        self.common
            .receivers
            .write()
            .unwrap()
            .remove(&ReceiverInnerPointer(&*self.inner));
    }
}
