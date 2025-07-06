use futures::{
    stream::{FusedStream, Stream},
    task::AtomicWaker,
};
use std::{
    self,
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
        if !self.flag.swap(true, Ordering::SeqCst) {
            self.waker.wake();
        }
    }

    pub fn sender(&self) -> Sender<'_> {
        Sender::new(self)
    }
    pub fn receiver(&self) -> Receiver<'_> {
        Receiver::new(self)
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
}
impl<'s> Receiver<'s> {
    fn new(parent: &'s Signal) -> Self {
        assert!(
            !parent.receiver_taken.swap(true, Ordering::SeqCst),
            "receiver already taken"
        );
        Self { parent }
    }
}
impl Stream for Receiver<'_> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        self_.parent.waker.register(cx.waker());

        let pending = self_.parent.flag.swap(false, Ordering::SeqCst);

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
        assert!(
            self.parent.receiver_taken.swap(false, Ordering::SeqCst),
            "receiver not taken?"
        );
    }
}
