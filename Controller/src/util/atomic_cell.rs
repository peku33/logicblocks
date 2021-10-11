use std::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct AtomicCellLease<'a, T>
where
    T: Send,
{
    parent: &'a AtomicCell<T>,
}
impl<'a, T> AtomicCellLease<'a, T>
where
    T: Send,
{
    fn new(parent: &'a AtomicCell<T>) -> Self {
        assert!(
            !parent.borrowed.swap(true, Ordering::Relaxed),
            "already borrowed"
        );
        Self { parent }
    }
}
impl<'a, T> Deref for AtomicCellLease<'a, T>
where
    T: Send,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.parent.inner.get() }
    }
}
impl<'a, T> DerefMut for AtomicCellLease<'a, T>
where
    T: Send,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.parent.inner.get() }
    }
}
impl<'a, T> fmt::Debug for AtomicCellLease<'a, T>
where
    T: Send,
    T: fmt::Debug,
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        self.deref().fmt(f)
    }
}
impl<'a, T> Drop for AtomicCellLease<'a, T>
where
    T: Send,
{
    fn drop(&mut self) {
        assert!(
            self.parent.borrowed.swap(false, Ordering::Relaxed),
            "not borrowed?"
        );
    }
}
unsafe impl<'a, T> Send for AtomicCellLease<'a, T> where T: Send {}

pub struct AtomicCell<T>
where
    T: Send,
{
    inner: UnsafeCell<T>,
    borrowed: AtomicBool,
}
impl<T> AtomicCell<T>
where
    T: Send,
{
    pub fn new(inner: T) -> Self {
        let inner = UnsafeCell::new(inner);

        let borrowed = false;
        let borrowed = AtomicBool::new(borrowed);

        Self { inner, borrowed }
    }
    pub fn lease(&self) -> AtomicCellLease<T> {
        AtomicCellLease::new(self)
    }
    pub fn get(&mut self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
}
impl<T> fmt::Debug for AtomicCell<T>
where
    T: Send,
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "AtomicCell {{ ... }}")
    }
}
unsafe impl<T> Send for AtomicCell<T> where T: Send {}
unsafe impl<T> Sync for AtomicCell<T> where T: Send {}
