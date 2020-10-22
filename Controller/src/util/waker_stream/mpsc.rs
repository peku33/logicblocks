use crate::util::atomic_cell::{AtomicCell, AtomicCellLease};
use futures::{stream::FusedStream, task::AtomicWaker, Stream};
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct Common {
    signaled: AtomicBool,
    waker: AtomicWaker,
}
impl Common {
    fn new() -> Self {
        Self {
            signaled: AtomicBool::new(false),
            waker: AtomicWaker::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Sender {
    common: Arc<Common>,
}
impl Sender {
    fn new(common: Arc<Common>) -> Self {
        Self { common }
    }
    pub fn wake(&self) {
        self.common.signaled.store(true, Ordering::Relaxed);
        self.common.waker.wake();
    }
}

#[derive(Debug)]
pub struct Receiver {
    common: Arc<Common>,
}
impl Receiver {
    fn new(common: Arc<Common>) -> Self {
        Self { common }
    }
}
impl Stream for Receiver {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        self.common.waker.register(cx.waker());
        let signaled = self.common.signaled.swap(false, Ordering::Relaxed);
        if signaled {
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

pub fn channel() -> (Sender, Receiver) {
    let common = Common::new();
    let common = Arc::new(common);

    let sender = Sender::new(common.clone());
    let receiver = Receiver::new(common);

    (sender, receiver)
}

// SenderReceiver
pub type ReceiverLease<'a> = AtomicCellLease<'a, Receiver>;
#[derive(Debug)]
pub struct SenderReceiver {
    sender: Sender,
    receiver_cell: AtomicCell<Receiver>,
}
impl SenderReceiver {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        let receiver_cell = AtomicCell::new(receiver);
        Self {
            sender,
            receiver_cell,
        }
    }

    pub fn wake(&self) {
        self.sender.wake();
    }

    pub fn receiver(&self) -> ReceiverLease {
        self.receiver_cell.lease()
    }
}
