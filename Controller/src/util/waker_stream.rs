use futures::{
    stream::{FusedStream, Stream},
    task::{AtomicWaker, Context, Poll},
};
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[derive(Debug)]
struct Inner {
    version: AtomicUsize,
    waker: AtomicWaker,
}
impl Inner {
    pub fn new() -> Self {
        Self {
            version: AtomicUsize::new(0),
            waker: AtomicWaker::new(),
        }
    }
}

#[derive(Debug)]
pub struct Sender {
    inner: Arc<Inner>,
}
impl Sender {
    pub fn new() -> Self {
        let inner = Inner::new();
        Self {
            inner: Arc::new(inner),
        }
    }
    pub fn wake(&self) {
        self.inner.version.fetch_add(1, Ordering::Relaxed);
        self.inner.waker.wake();
    }
    pub fn receiver(&self) -> Receiver {
        Receiver::new(self.inner.clone())
    }
}

#[derive(Debug)]
pub struct Receiver {
    inner: Arc<Inner>,
    version: AtomicUsize,
}
impl Receiver {
    fn new(inner: Arc<Inner>) -> Self {
        let version = inner.version.load(Ordering::Relaxed);
        Self {
            inner,
            version: AtomicUsize::new(version),
        }
    }
}
impl Stream for Receiver {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        self_.inner.waker.register(cx.waker());

        let version = self_.inner.version.load(Ordering::SeqCst);
        if self_.version.swap(version, Ordering::Relaxed) != version {
            return Poll::Ready(Some(()));
        }
        Poll::Pending
    }
}
impl FusedStream for Receiver {
    fn is_terminated(&self) -> bool {
        false
    }
}
