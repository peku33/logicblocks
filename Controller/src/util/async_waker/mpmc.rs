use futures::{Stream, stream::FusedStream, task::AtomicWaker};
use parking_lot::RwLock;
use std::{
    collections::HashSet,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct Signal {
    receiver_inners: RwLock<HashSet<*const ReceiverInner>>,
}
impl Signal {
    pub fn new() -> Self {
        let receiver_inners = HashSet::<*const ReceiverInner>::new();
        let receiver_inners = RwLock::new(receiver_inners);

        Self { receiver_inners }
    }

    pub fn wake(&self) {
        self.receiver_inners
            .read()
            .iter()
            .copied()
            .for_each(|receiver_inner| {
                let receiver_inner = unsafe { &*receiver_inner };
                if !receiver_inner.flag.swap(true, Ordering::SeqCst) {
                    receiver_inner.waker.wake();
                }
            });
    }

    pub fn sender(&self) -> Sender {
        Sender::new(self)
    }
    pub fn receiver(&self) -> Receiver {
        Receiver::new(self)
    }
}
impl Drop for Signal {
    fn drop(&mut self) {
        debug_assert!(self.receiver_inners.read().is_empty());
    }
}
unsafe impl Send for Signal {}
unsafe impl Sync for Signal {}

#[derive(Debug)]
pub struct Sender<'s> {
    signal: &'s Signal,
}
impl<'s> Sender<'s> {
    fn new(signal: &'s Signal) -> Self {
        Self { signal }
    }

    pub fn wake(&self) {
        self.signal.wake();
    }
}

#[derive(Debug)]
struct ReceiverInner {
    waker: AtomicWaker,
    flag: AtomicBool,
}
#[derive(Debug)]
pub struct Receiver<'s> {
    signal: &'s Signal,
    inner: Pin<Box<ReceiverInner>>,
}
impl<'s> Receiver<'s> {
    fn new(signal: &'s Signal) -> Self {
        let waker = AtomicWaker::new();

        let flag = false;
        let flag = AtomicBool::new(flag);

        let inner = ReceiverInner { waker, flag };
        let inner = Box::pin(inner);

        let receiver_inner_ptr = &*inner as *const ReceiverInner;
        let inserted = signal.receiver_inners.write().insert(receiver_inner_ptr);
        debug_assert!(inserted);

        Self { signal, inner }
    }
}
impl Stream for Receiver<'_> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        self_.inner.waker.register(cx.waker());

        let pending = self_.inner.flag.swap(false, Ordering::SeqCst);

        if pending {
            Poll::Ready(Some(()))
        } else {
            Poll::Pending
        }
    }
}
impl FusedStream for Receiver<'_> {
    fn is_terminated(&self) -> bool {
        false
    }
}
impl Drop for Receiver<'_> {
    fn drop(&mut self) {
        let receiver_inner_ptr = &*self.inner as *const ReceiverInner;
        let removed = self
            .signal
            .receiver_inners
            .write()
            .remove(&receiver_inner_ptr);
        debug_assert!(removed);
    }
}
