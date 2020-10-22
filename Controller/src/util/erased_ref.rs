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

pub struct ErasedRef<T> {
    inner: Pin<Box<Inner<T>>>,
}
impl<T: fmt::Debug> fmt::Debug for ErasedRef<T> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("ErasedRef")
            .field("value", &self.inner.value)
            .field(
                "lease_count",
                &self.inner.lease_count.load(Ordering::Relaxed),
            )
            .finish()
    }
}
impl<T> ErasedRef<T> {
    pub fn new(value: T) -> Self {
        let inner = Inner {
            value,
            lease_count: AtomicUsize::new(0),
        };
        let inner = Box::pin(inner);
        Self { inner }
    }

    pub fn lease(&self) -> ErasedRefLease<T> {
        ErasedRefLease::new(self)
    }
}
impl<T> Deref for ErasedRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}
impl<T> Drop for ErasedRef<T> {
    fn drop(&mut self) {
        if self.inner.lease_count.load(Ordering::Relaxed) != 0 {
            panic!("dropping ErasedRef while ErasedRefLease still exists");
        }
    }
}

pub struct ErasedRefLease<T> {
    inner: *const Inner<T>,
}
unsafe impl<T: Send> Send for ErasedRefLease<T> {}
unsafe impl<T: Sync> Sync for ErasedRefLease<T> {}
impl<T> ErasedRefLease<T> {
    fn new(parent: &ErasedRef<T>) -> Self {
        parent.inner.lease_count.fetch_add(1, Ordering::Relaxed);

        let inner: *const Inner<T> = &*parent.inner;
        Self { inner }
    }
}
impl<T: fmt::Debug> fmt::Debug for ErasedRefLease<T> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let inner = unsafe { &(*self.inner) };

        f.debug_struct("ErasedRefLease")
            .field("value", &inner.value)
            .field("lease_count", &inner.lease_count.load(Ordering::Relaxed))
            .finish()
    }
}
impl<T> Deref for ErasedRefLease<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let inner = unsafe { &(*self.inner) };
        &inner.value
    }
}
impl<T> Drop for ErasedRefLease<T> {
    fn drop(&mut self) {
        let inner = unsafe { &(*self.inner) };
        inner.lease_count.fetch_sub(1, Ordering::Relaxed);
    }
}
