use futures::{stream::FusedStream, task::AtomicWaker, Stream};
use parking_lot::RwLock;
use std::{
    collections::HashSet,
    pin::Pin,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

const STATE_SHIFT_SIGNALED: u8 = 0;
const STATE_SHIFT_TERMINATED: u8 = 1;

struct Common {
    receiver_inners: RwLock<HashSet<*const ReceiverInner>>,
}
impl Common {
    fn new() -> Self {
        let receiver_inners = HashSet::<*const ReceiverInner>::new();
        let receiver_inners = RwLock::new(receiver_inners);
        Self { receiver_inners }
    }
}
unsafe impl Send for Common {}
unsafe impl Sync for Common {}

pub struct Sender {
    common: Arc<Common>,
}
impl Sender {
    pub fn new() -> Self {
        let common = Common::new();
        let common = Arc::new(common);
        Self { common }
    }

    pub fn wake(&self) {
        self.common
            .receiver_inners
            .read()
            .iter()
            .for_each(|receiver_inner| {
                let receiver_inner = unsafe { &**receiver_inner };

                if receiver_inner
                    .state
                    .fetch_or(1 << STATE_SHIFT_SIGNALED, Ordering::SeqCst)
                    & (1 << STATE_SHIFT_SIGNALED)
                    == 0
                {
                    receiver_inner.waker.wake();
                }
            });
    }

    pub fn receiver(&self) -> Receiver {
        Receiver::new(self.common.clone())
    }
}
impl Drop for Sender {
    fn drop(&mut self) {
        self.common
            .receiver_inners
            .read()
            .iter()
            .for_each(|receiver_inner| {
                let receiver_inner = unsafe { &**receiver_inner };

                if receiver_inner
                    .state
                    .fetch_or(1 << STATE_SHIFT_TERMINATED, Ordering::SeqCst)
                    & (1 << STATE_SHIFT_TERMINATED)
                    == 0
                {
                    receiver_inner.waker.wake();
                }
            });
    }
}

struct ReceiverInner {
    waker: AtomicWaker,
    state: AtomicU8,
}
pub struct Receiver {
    common: Arc<Common>,
    inner: Pin<Box<ReceiverInner>>,
}
impl Receiver {
    fn new(common: Arc<Common>) -> Self {
        let waker = AtomicWaker::new();

        let state = 0;
        let state = AtomicU8::new(state);

        let inner = ReceiverInner { waker, state };
        let inner = Box::pin(inner);

        let receiver_inner_ptr = &*inner as *const ReceiverInner;
        let inserted = common.receiver_inners.write().insert(receiver_inner_ptr);
        debug_assert!(inserted);

        Self { common, inner }
    }
}
impl Stream for Receiver {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        self_.inner.waker.register(cx.waker());

        let state = self_
            .inner
            .state
            .fetch_and(!(1 << STATE_SHIFT_SIGNALED), Ordering::SeqCst);

        if state & (1 << STATE_SHIFT_TERMINATED) != 0 {
            Poll::Ready(None)
        } else if state & (1 << STATE_SHIFT_SIGNALED) != 0 {
            Poll::Ready(Some(()))
        } else {
            Poll::Pending
        }
    }
}
impl FusedStream for Receiver {
    fn is_terminated(&self) -> bool {
        self.inner.state.load(Ordering::SeqCst) & (1 << STATE_SHIFT_TERMINATED) != 0
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        let receiver_inner_ptr = &*self.inner as *const ReceiverInner;
        let removed = self
            .common
            .receiver_inners
            .write()
            .remove(&receiver_inner_ptr);
        debug_assert!(removed);
    }
}
