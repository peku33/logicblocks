use futures::{Future, future::FusedFuture, task::AtomicWaker};
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct Barrier {
    released: AtomicBool,
    children: Mutex<HashSet<*const WaiterInnerWaiting>>,
}
impl Barrier {
    pub fn new() -> Self {
        let released = false;
        let released = AtomicBool::new(released);

        let children = HashSet::<*const WaiterInnerWaiting>::new();
        let children = Mutex::new(children);

        Self { released, children }
    }

    pub fn release(&self) {
        self.released.store(true, Ordering::Relaxed);

        let children = self.children.lock();
        for child in children.iter() {
            let child = unsafe { &**child };
            child.waker.wake();
        }
    }

    pub async fn waiter(&self) -> Waiter<'_> {
        Waiter::new(self)
    }
}
unsafe impl Send for Barrier {}
unsafe impl Sync for Barrier {}

#[derive(Debug)]
struct WaiterInnerWaiting {
    waker: AtomicWaker,
}
impl WaiterInnerWaiting {
    fn new() -> Self {
        let waker = AtomicWaker::new();
        Self { waker }
    }
}
#[derive(Debug)]
enum WaiterInner {
    Released,
    Waiting { inner: Pin<Box<WaiterInnerWaiting>> },
}
#[derive(Debug)]
pub struct Waiter<'p> {
    parent: &'p Barrier,
    inner: WaiterInner,
}
impl<'p> Waiter<'p> {
    pub fn new(parent: &'p Barrier) -> Self {
        let released = parent.released.load(Ordering::Relaxed);

        let inner = if released {
            WaiterInner::Released
        } else {
            let inner = WaiterInnerWaiting::new();
            let inner = Box::pin(inner);

            let mut children = parent.children.lock();
            assert!(children.insert(&*inner as *const WaiterInnerWaiting));
            drop(children);

            WaiterInner::Waiting { inner }
        };

        Self { parent, inner }
    }
}
impl Future for Waiter<'_> {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let self_ = unsafe { self.get_unchecked_mut() };

        match &mut self_.inner {
            WaiterInner::Released => Poll::Ready(()),
            WaiterInner::Waiting { inner } => {
                if self_.parent.released.load(Ordering::Relaxed) {
                    Poll::Ready(())
                } else {
                    inner.waker.register(cx.waker());
                    Poll::Pending
                }
            }
        }
    }
}
impl FusedFuture for Waiter<'_> {
    fn is_terminated(&self) -> bool {
        match self.inner {
            WaiterInner::Released => true,
            WaiterInner::Waiting { .. } => self.parent.released.load(Ordering::Relaxed),
        }
    }
}
impl Drop for Waiter<'_> {
    fn drop(&mut self) {
        if let WaiterInner::Waiting { inner } = &mut self.inner {
            let mut children = self.parent.children.lock();
            assert!(children.remove(&(&**inner as *const WaiterInnerWaiting)));
            drop(children);
        }
    }
}
