use futures::{
    future::{FusedFuture, Future},
    stream::{FusedStream, Stream},
    task::AtomicWaker,
};
use parking_lot::RwLock;
use std::{
    collections::HashSet,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
struct Inner<T>
where
    T: Clone + PartialEq + Eq,
{
    value: T,
    remotes: HashSet<*const InnerRemote>,
}
unsafe impl<T> Send for Inner<T> where T: Clone + PartialEq + Eq {}
unsafe impl<T> Sync for Inner<T> where T: Clone + PartialEq + Eq {}

impl<T> Inner<T>
where
    T: Clone + PartialEq + Eq,
{
    pub fn new(initial: T) -> Self {
        #[allow(clippy::mutable_key_type)]
        let remotes = HashSet::new();

        Self {
            value: initial,
            remotes,
        }
    }
}

#[derive(Debug)]
struct InnerRemote {
    waker: AtomicWaker,
}
impl InnerRemote {
    fn new() -> Self {
        let waker = AtomicWaker::new();
        Self { waker }
    }
}

// Value
/// Main value holder object.
/// This should be but in the place where you manage contained value
#[derive(Debug)]
pub struct Value<T>
where
    T: Clone + PartialEq + Eq,
{
    inner: RwLock<Inner<T>>,
}
impl<T> Value<T>
where
    T: Clone + PartialEq + Eq,
{
    /// Initializes object with given initial value
    pub fn new(initial: T) -> Self {
        let inner = Inner::new(initial);
        let inner = RwLock::new(inner);
        Self { inner }
    }

    /// Returns current value
    pub fn get(&self) -> T {
        self.inner.read().value.clone()
    }
    /// Sets new value
    /// Returns false if value was equal to previous and was not updated
    /// Returns true if value was updated and observers were waked
    pub fn set(
        &self,
        value: T,
    ) -> bool {
        let mut inner = self.inner.write();
        let changed = if inner.value != value {
            inner.value = value;
            inner.remotes.iter().copied().for_each(|remote| {
                let remote = unsafe { &*remote };
                remote.waker.wake();
            });
            true
        } else {
            false
        };
        drop(inner);

        changed
    }

    /// Returns [`Getter`]
    pub fn getter(&self) -> Getter<'_, T> {
        Getter::new(self)
    }
    /// Returns [`Setter`]
    pub fn setter(&self) -> Setter<'_, T> {
        Setter::new(self)
    }

    /// Returns [`Observer`]
    pub fn observer(
        &self,
        initially_pending: bool,
    ) -> Observer<'_, T> {
        Observer::new(self, initially_pending)
    }
    /// Returns [`ChangedStream`]
    pub fn changed_stream(
        &self,
        initially_pending: bool,
    ) -> ChangedStream<'_, T> {
        ChangedStream::new(self, initially_pending)
    }
    /// Returns [`ValueStream`]
    pub fn value_stream(
        &self,
        initially_pending: bool,
    ) -> ValueStream<'_, T> {
        ValueStream::new(self, initially_pending)
    }
}

// Getter
/// Getter object - read only part od [`Value`]. Creating and keeping this object has no cost, it's just a proxy around
/// read-only methods of [`Value`].
#[derive(Debug)]
pub struct Getter<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    parent: &'v Value<T>,
}
impl<'v, T> Getter<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn new(parent: &'v Value<T>) -> Self {
        Self { parent }
    }

    /// Returns current value
    pub fn get(&self) -> T {
        self.parent.get()
    }

    /// Returns [`Observer`]
    pub fn observer(
        &self,
        initially_pending: bool,
    ) -> Observer<'v, T> {
        self.parent.observer(initially_pending)
    }
    /// Returns [`ChangedStream`]
    pub fn changed_stream(
        &self,
        initially_pending: bool,
    ) -> ChangedStream<'v, T> {
        self.parent.changed_stream(initially_pending)
    }
    /// Returns [`ValueStream`]
    pub fn value_stream(
        &self,
        initially_pending: bool,
    ) -> ValueStream<'v, T> {
        self.parent.value_stream(initially_pending)
    }
}

// Setter
/// Setter object - read-write part od [`Value`]. Creating and keeping this object has no cost, it's just a proxy around
/// read-write methods of [`Value`].
#[derive(Debug)]
pub struct Setter<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    parent: &'v Value<T>,
}
impl<'v, T> Setter<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn new(parent: &'v Value<T>) -> Self {
        Self { parent }
    }

    /// Returns current value
    pub fn get(&self) -> T {
        self.parent.get()
    }
    /// Sets new value
    /// Returns false if value was equal to previous and was not updated
    /// Returns true if value was updated and observers were waked
    pub fn set(
        &mut self,
        value: T,
    ) -> bool {
        self.parent.set(value)
    }

    /// Returns [`Observer`]
    pub fn observer(
        &self,
        initially_pending: bool,
    ) -> Observer<'v, T> {
        self.parent.observer(initially_pending)
    }
    /// Returns [`ChangedStream`]
    pub fn changed_stream(
        &self,
        initially_pending: bool,
    ) -> ChangedStream<'v, T> {
        self.parent.changed_stream(initially_pending)
    }
    /// Returns [`ValueStream`]
    pub fn value_stream(
        &self,
        initially_pending: bool,
    ) -> ValueStream<'v, T> {
        self.parent.value_stream(initially_pending)
    }
}

// Observer
/// Observer object. This internally keeps the copy of "last seen" or "last processed" value. Calling its method will
/// compare this value against global (from [`Value`]) value and tell whether it was changed.
#[derive(Debug)]
pub struct Observer<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    parent: &'v Value<T>,
    last_seen_value: Option<T>,
}
impl<'v, T> Observer<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn new(
        parent: &'v Value<T>,
        initially_pending: bool,
    ) -> Self {
        let last_seen_value = if !initially_pending {
            Some(parent.inner.read().value.clone())
        } else {
            None
        };

        Self {
            parent,
            last_seen_value,
        }
    }

    /// Returns latest global (from [`Value`]) value and marks it as "last seen".
    pub fn get_update(&mut self) -> &T {
        let parent_inner = self.parent.inner.read();

        if !self.last_seen_value.contains(&parent_inner.value) {
            self.last_seen_value.replace(parent_inner.value.clone());
        }

        drop(parent_inner);

        self.last_seen_value.as_ref().unwrap()
    }

    /// Returns latest global (from [`Value`]) value and marks it as "last seen" only if it differs from previous
    /// "last seen".
    pub fn get_changed_update(&mut self) -> Option<&T> {
        let parent_inner = self.parent.inner.read();

        let changed = if !self.last_seen_value.contains(&parent_inner.value) {
            self.last_seen_value.replace(parent_inner.value.clone());
            Some(self.last_seen_value.as_ref().unwrap())
        } else {
            None
        };

        drop(parent_inner);

        changed
    }

    /// Returns [`ObserverCommitter`] object if global (from [`Value`]) value differs from "last seen".
    pub fn get_changed_committer(&mut self) -> Option<ObserverCommitter<'_, 'v, T>> {
        let parent_inner = self.parent.inner.read();

        let observer_committer = if !self.last_seen_value.contains(&parent_inner.value) {
            Some(ObserverCommitter::new(self, parent_inner.value.clone()))
        } else {
            None
        };

        drop(parent_inner);

        observer_committer
    }

    /// Returns the [`ObserverChanged`] object.
    pub fn changed(&mut self) -> ObserverChanged<'_, 'v, T> {
        ObserverChanged::new(self)
    }
}

/// This object allows to first freeze the value and then mark it (or not) as "last seen" or "last processed". This
/// is useful if processing of the value may fail and we may want to retry later, keeping pending state.
pub struct ObserverCommitter<'r, 'v, T>
where
    T: Clone + PartialEq + Eq,
{
    parent: &'r mut Observer<'v, T>,
    pending_value: T,
}
impl<'r, 'v, T> ObserverCommitter<'r, 'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn new(
        parent: &'r mut Observer<'v, T>,
        pending_value: T,
    ) -> Self {
        Self {
            parent,
            pending_value,
        }
    }

    /// Returns value. The value is frozen inside, meaning it won't change between calls, event if [`Value`] value was
    /// updated.
    pub fn value(&self) -> &T {
        &self.pending_value
    }
    /// Marks value returned by [`Self::value`] as last seen.
    pub fn commit(self) {
        self.parent.last_seen_value.replace(self.pending_value);
    }
}

// ObserverChanged
/// Future that will complete if value stored in [`Observer`] (aka last seen) differs from value stored
/// in [`Value`] (aka global).
#[derive(Debug)]
pub struct ObserverChanged<'r, 'v, T>
where
    T: Clone + PartialEq + Eq,
{
    parent: &'r mut Observer<'v, T>,
    inner_remote: Pin<Box<InnerRemote>>,
    competed: bool,
}
impl<'r, 'v, T> ObserverChanged<'r, 'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn new(parent: &'r mut Observer<'v, T>) -> Self {
        let inner_remote = InnerRemote::new();
        let inner_remote = Box::pin(inner_remote);

        let inserted = parent
            .parent
            .inner
            .write()
            .remotes
            .insert(&*inner_remote as *const InnerRemote);
        assert!(inserted);

        let competed = false;

        Self {
            parent,
            inner_remote,
            competed,
        }
    }
}
impl<'r, 'v, T> Future for ObserverChanged<'r, 'v, T>
where
    T: Clone + PartialEq + Eq,
{
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let self_ = unsafe { self.get_unchecked_mut() };

        debug_assert!(!self_.competed);

        let self_parent_parent_inner = self_.parent.parent.inner.read();

        let poll = if !self_
            .parent
            .last_seen_value
            .contains(&self_parent_parent_inner.value)
        {
            self_.competed = true;
            Poll::Ready(())
        } else {
            self_.inner_remote.waker.register(cx.waker());
            Poll::Pending
        };

        drop(self_parent_parent_inner);

        poll
    }
}
impl<'r, 'v, T> FusedFuture for ObserverChanged<'r, 'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn is_terminated(&self) -> bool {
        self.competed
    }
}
impl<'r, 'v, T> Drop for ObserverChanged<'r, 'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn drop(&mut self) {
        let removed = self
            .parent
            .parent
            .inner
            .write()
            .remotes
            .remove(&(&*self.inner_remote as *const InnerRemote));
        assert!(removed);
    }
}

// ChangedStream
/// Stream yielding () when value is changed. Remembers value from previous yield and yields only if value differs. If
/// value went A -> B -> A quickly, it is possible it won't yield. Use this when you don't need value returned from
/// stream, as this is a bit faster then [`ValueStream`].
pub struct ChangedStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    parent: &'v Value<T>,
    inner_remote: Pin<Box<InnerRemote>>,
    last_seen_value: Option<T>,
}
impl<'v, T> ChangedStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn new(
        parent: &'v Value<T>,
        initially_pending: bool,
    ) -> Self {
        let inner_remote = InnerRemote::new();
        let inner_remote = Box::pin(inner_remote);

        let mut parent_inner = parent.inner.write();

        let inserted = parent_inner
            .remotes
            .insert(&*inner_remote as *const InnerRemote);
        assert!(inserted);

        let last_seen_value = if !initially_pending {
            Some(parent_inner.value.clone())
        } else {
            None
        };

        drop(parent_inner);

        Self {
            parent,
            inner_remote,
            last_seen_value,
        }
    }
}
impl<'v, T> Stream for ChangedStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        let parent_inner = self_.parent.inner.read();

        let poll = if !self_.last_seen_value.contains(&parent_inner.value) {
            self_.last_seen_value.replace(parent_inner.value.clone());
            Poll::Ready(Some(()))
        } else {
            self_.inner_remote.waker.register(cx.waker());
            Poll::Pending
        };

        drop(parent_inner);

        poll
    }
}
impl<'v, T> FusedStream for ChangedStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn is_terminated(&self) -> bool {
        false
    }
}
impl<'v, T> Drop for ChangedStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn drop(&mut self) {
        let removed = self
            .parent
            .inner
            .write()
            .remotes
            .remove(&(&*self.inner_remote as *const InnerRemote));
        assert!(removed);
    }
}

// ValueStream
/// Stream yielding value when it is changed. Remembers value from previous yield and yields only if value differs. If
/// value went A -> B -> A quickly, it is possible it won't yield. A bit slower then [`ChangedStream`], because of
/// additional clone, so use it only when you actually need value from stream, not just information it was changed.
pub struct ValueStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    parent: &'v Value<T>,
    inner_remote: Pin<Box<InnerRemote>>,
    last_seen_value: Option<T>,
}
impl<'v, T> ValueStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn new(
        parent: &'v Value<T>,
        initially_pending: bool,
    ) -> Self {
        let inner_remote = InnerRemote::new();
        let inner_remote = Box::pin(inner_remote);

        let mut parent_inner = parent.inner.write();

        let inserted = parent_inner
            .remotes
            .insert(&*inner_remote as *const InnerRemote);
        assert!(inserted);

        let last_seen_value = if !initially_pending {
            Some(parent_inner.value.clone())
        } else {
            None
        };

        drop(parent_inner);

        Self {
            parent,
            inner_remote,
            last_seen_value,
        }
    }
}
impl<'v, T> Stream for ValueStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    type Item = T;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        let parent_inner = self_.parent.inner.read();

        let poll = if !self_.last_seen_value.contains(&parent_inner.value) {
            self_.last_seen_value.replace(parent_inner.value.clone());
            Poll::Ready(Some(parent_inner.value.clone()))
        } else {
            self_.inner_remote.waker.register(cx.waker());
            Poll::Pending
        };

        drop(parent_inner);

        poll
    }
}
impl<'v, T> FusedStream for ValueStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn is_terminated(&self) -> bool {
        false
    }
}
impl<'v, T> Drop for ValueStream<'v, T>
where
    T: Clone + PartialEq + Eq,
{
    fn drop(&mut self) {
        let removed = self
            .parent
            .inner
            .write()
            .remotes
            .remove(&(&*self.inner_remote as *const InnerRemote));
        assert!(removed);
    }
}
