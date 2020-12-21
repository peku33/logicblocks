use std::{
    fmt,
    ops::Deref,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
struct Inner<T> {
    value: T,
    lease_count: AtomicUsize,
}

pub struct AtomicCellErased<T> {
    inner: Pin<Box<Inner<T>>>,
}
impl<T: fmt::Debug> fmt::Debug for AtomicCellErased<T> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("AtomicCellErased")
            .field("value", &self.inner.value)
            .field(
                "lease_count",
                &self.inner.lease_count.load(Ordering::Relaxed),
            )
            .finish()
    }
}
impl<T> AtomicCellErased<T> {
    pub fn new(value: T) -> Self {
        let inner = Inner {
            value,
            lease_count: AtomicUsize::new(0),
        };
        let inner = Box::pin(inner);
        Self { inner }
    }

    pub fn lease(&self) -> AtomicCellErasedLease<T> {
        AtomicCellErasedLease::new(self)
    }
}
impl<T> Deref for AtomicCellErased<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}
impl<T> Drop for AtomicCellErased<T> {
    fn drop(&mut self) {
        if self.inner.lease_count.load(Ordering::Relaxed) != 0 {
            panic!("dropping AtomicCellErased while AtomicCellErasedLease still exists");
        }
    }
}

pub struct AtomicCellErasedLease<T> {
    inner: *const Inner<T>,
}
unsafe impl<T: Send> Send for AtomicCellErasedLease<T> {}
unsafe impl<T: Sync> Sync for AtomicCellErasedLease<T> {}
impl<T> AtomicCellErasedLease<T> {
    fn new(parent: &AtomicCellErased<T>) -> Self {
        parent.inner.lease_count.fetch_add(1, Ordering::Relaxed);

        let inner: *const Inner<T> = &*parent.inner;
        Self { inner }
    }
}
impl<T: fmt::Debug> fmt::Debug for AtomicCellErasedLease<T> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let inner = unsafe { &(*self.inner) };

        f.debug_struct("AtomicCellErasedLease")
            .field("value", &inner.value)
            .field("lease_count", &inner.lease_count.load(Ordering::Relaxed))
            .finish()
    }
}
impl<T> Deref for AtomicCellErasedLease<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let inner = unsafe { &(*self.inner) };
        &inner.value
    }
}
impl<T> Drop for AtomicCellErasedLease<T> {
    fn drop(&mut self) {
        let inner = unsafe { &(*self.inner) };
        inner.lease_count.fetch_sub(1, Ordering::Relaxed);
    }
}
