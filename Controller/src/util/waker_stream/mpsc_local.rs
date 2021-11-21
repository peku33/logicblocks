use futures::{
    stream::{FusedStream, Stream},
    task::AtomicWaker,
};
use std::{
    self,
    mem::replace,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct Signal {
    flag: AtomicBool,
    waker: AtomicWaker,

    receiver_taken: AtomicBool,
}
impl Signal {
    pub fn new() -> Self {
        let flag = false;
        let flag = AtomicBool::new(flag);

        let waker = AtomicWaker::new();

        let receiver_taken = false;
        let receiver_taken = AtomicBool::new(receiver_taken);

        Self {
            flag,
            waker,
            receiver_taken,
        }
    }

    pub fn wake(&self) {
        self.flag.store(true, Ordering::Relaxed);
        self.waker.wake();
    }

    pub fn sender(&self) -> Sender {
        Sender::new(self)
    }
    pub fn receiver(
        &self,
        initially_pending: bool,
    ) -> Receiver {
        Receiver::new(self, initially_pending)
    }
}

#[derive(Debug)]
pub struct Sender<'s> {
    parent: &'s Signal,
}
impl<'s> Sender<'s> {
    fn new(parent: &'s Signal) -> Self {
        Self { parent }
    }

    pub fn wake(&self) {
        self.parent.wake();
    }
}

#[derive(Debug)]
pub struct Receiver<'s> {
    parent: &'s Signal,
    force_pending: bool,
}
impl<'s> Receiver<'s> {
    fn new(
        parent: &'s Signal,
        initially_pending: bool,
    ) -> Self {
        assert!(
            !parent.receiver_taken.swap(true, Ordering::Relaxed),
            "receiver already taken"
        );
        Self {
            parent,
            force_pending: initially_pending,
        }
    }
}
impl<'s> Stream for Receiver<'s> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        self_.parent.waker.register(cx.waker());

        let real_pending = self_.parent.flag.swap(false, Ordering::Relaxed);
        let force_pending = replace(&mut self_.force_pending, false);

        if real_pending || force_pending {
            Poll::Ready(Some(()))
        } else {
            Poll::Pending
        }
    }
}
impl<'s> FusedStream for Receiver<'s> {
    fn is_terminated(&self) -> bool {
        false
    }
}
impl<'s> Drop for Receiver<'s> {
    fn drop(&mut self) {
        assert!(
            self.parent.receiver_taken.swap(false, Ordering::Relaxed),
            "receiver not taken?"
        );
    }
}
