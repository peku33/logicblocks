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

#[derive(Debug)]
struct Inner {
    signaled: AtomicBool,
    receivers: Mutex<HashSet<*const ReceiverInner>>,
}
impl Inner {
    pub fn new() -> Self {
        let signaled = false;
        let signaled = AtomicBool::new(signaled);

        #[allow(clippy::mutable_key_type)]
        let receivers = HashSet::new();
        let receivers = Mutex::new(receivers);

        Self {
            signaled,
            receivers,
        }
    }

    pub fn signal(&self) {
        debug_assert!(
            !self.signaled.swap(true, Ordering::Relaxed),
            "flag already signaled",
        );

        self.receivers
            .lock()
            .iter()
            .copied()
            .for_each(|receiver_inner_ptr| {
                let receiver_inner = unsafe { &*receiver_inner_ptr };
                receiver_inner.waker.wake();
            });
    }
}
unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

#[derive(Debug)]
pub struct Sender {
    inner: Arc<Inner>,
}
impl Sender {
    pub fn new() -> Self {
        let inner = Inner::new();
        let inner = Arc::new(inner);

        Self { inner }
    }
    pub fn receiver(&self) -> Receiver {
        Receiver::new(self.inner.clone())
    }

    pub fn signal(self) {
        self.inner.signal();
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

pub fn pair() -> (Sender, Receiver) {
    let sender = Sender::new();
    let receiver = sender.receiver();
    (sender, receiver)
}

pub struct LocalSender {
    signaled: AtomicBool,
    receivers: Mutex<HashSet<*const LocalReceiverInner>>,
}
impl LocalSender {
    pub fn new() -> Self {
        let signaled = false;
        let signaled = AtomicBool::new(signaled);

        #[allow(clippy::mutable_key_type)]
        let receivers = HashSet::new();
        let receivers = Mutex::new(receivers);

        Self {
            signaled,
            receivers,
        }
    }

    pub fn receiver(&self) -> LocalReceiver<'_> {
        LocalReceiver::new(self)
    }

    pub fn signal(&self) {
        if self.signaled.swap(true, Ordering::Relaxed) {
            return;
        }

        self.receivers
            .lock()
            .iter()
            .copied()
            .for_each(|local_receiver_inner_ptr| {
                let local_receiver_inner = unsafe { &*local_receiver_inner_ptr };
                local_receiver_inner.waker.wake();
            });
    }
}
unsafe impl Send for LocalSender {}
unsafe impl Sync for LocalSender {}

struct LocalReceiverInner {
    waker: AtomicWaker,
}
impl LocalReceiverInner {
    pub fn new() -> Self {
        Self {
            waker: AtomicWaker::new(),
        }
    }
}
unsafe impl Send for LocalReceiverInner {}
unsafe impl Sync for LocalReceiverInner {}

pub struct LocalReceiver<'s> {
    sender: &'s LocalSender,
    receiver_inner: Pin<Box<LocalReceiverInner>>,
}
impl<'s> LocalReceiver<'s> {
    pub fn new(sender: &'s LocalSender) -> Self {
        let receiver_inner = Box::pin(LocalReceiverInner::new());

        assert!(sender
            .receivers
            .lock()
            .insert(&*receiver_inner as *const LocalReceiverInner));

        Self {
            sender,
            receiver_inner,
        }
    }
}
impl<'s> Future for LocalReceiver<'s> {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let self_ = unsafe { self.get_unchecked_mut() };

        self_.receiver_inner.waker.register(cx.waker());
        if self_.sender.signaled.load(Ordering::Relaxed) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
impl<'s> Drop for LocalReceiver<'s> {
    fn drop(&mut self) {
        assert!(self
            .sender
            .receivers
            .lock()
            .remove(&(&*self.receiver_inner as *const LocalReceiverInner)));
    }
}
