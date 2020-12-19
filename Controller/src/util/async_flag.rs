use futures::{
    future::{FusedFuture, Future},
    task::AtomicWaker,
};
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

struct Inner {
    signaled: AtomicBool,
    receivers: Mutex<HashSet<*const ReceiverInner>>,
}
impl Inner {
    pub fn new() -> Self {
        Self {
            signaled: AtomicBool::new(false),
            receivers: Mutex::new(HashSet::new()),
        }
    }
}
unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

pub struct Sender {
    inner: Arc<Inner>,
}
impl Sender {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner::new()),
        }
    }
    pub fn receiver(&self) -> Receiver {
        Receiver::new(self.inner.clone())
    }

    pub fn signal(self) {
        debug_assert!(
            !self.inner.signaled.swap(true, Ordering::Relaxed),
            "flag already signaled",
        );

        self.inner
            .receivers
            .lock()
            .iter()
            .copied()
            .for_each(|receiver_inner_ptr| {
                let receiver_inner = unsafe { &*receiver_inner_ptr };
                receiver_inner.waker.wake();
            });
    }
}

struct ReceiverInner {
    inner: Arc<Inner>,
    waker: AtomicWaker,
}
pub struct Receiver {
    completed: bool,
    receiver_inner: Pin<Box<ReceiverInner>>,
}
impl Receiver {
    fn new(inner: Arc<Inner>) -> Self {
        let receiver_inner = Box::pin(ReceiverInner {
            inner,
            waker: AtomicWaker::new(),
        });

        assert!(receiver_inner
            .inner
            .receivers
            .lock()
            .insert(&*receiver_inner as *const ReceiverInner));

        Self {
            completed: false,
            receiver_inner,
        }
    }
}
impl Clone for Receiver {
    fn clone(&self) -> Self {
        Self::new(self.receiver_inner.inner.clone())
    }
}
impl Future for Receiver {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let self_ = unsafe { self.get_unchecked_mut() };

        self_.receiver_inner.waker.register(cx.waker());
        if self_.receiver_inner.inner.signaled.load(Ordering::Relaxed) {
            self_.completed = true;
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
impl FusedFuture for Receiver {
    fn is_terminated(&self) -> bool {
        self.completed
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        assert!(self
            .receiver_inner
            .inner
            .receivers
            .lock()
            .remove(&(&*self.receiver_inner as *const ReceiverInner)));
    }
}
