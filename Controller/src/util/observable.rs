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
    pub fn new(initial: T) -> Self {
        let inner = Inner::new(initial);
        let inner = RwLock::new(inner);
        Self { inner }
    }

    pub fn get(&self) -> T {
        self.inner.read().value.clone()
    }
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

    pub fn getter(&self) -> Getter<'_, T> {
        Getter::new(self)
    }
    pub fn setter(&self) -> Setter<'_, T> {
        Setter::new(self)
    }

    pub fn observer(
        &self,
        initially_pending: bool,
    ) -> Observer<'_, T> {
        Observer::new(self, initially_pending)
    }
    pub fn changed_stream(
        &self,
        initially_pending: bool,
    ) -> ChangedStream<'_, T> {
        ChangedStream::new(self, initially_pending)
    }
    pub fn value_stream(
        &self,
        initially_pending: bool,
    ) -> ValueStream<'_, T> {
        ValueStream::new(self, initially_pending)
    }
}

// Getter
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

    pub fn get(&self) -> T {
        self.parent.get()
    }

    pub fn observer(
        &self,
        initially_pending: bool,
    ) -> Observer<'v, T> {
        self.parent.observer(initially_pending)
    }
    pub fn changed_stream(
        &self,
        initially_pending: bool,
    ) -> ChangedStream<'v, T> {
        self.parent.changed_stream(initially_pending)
    }
    pub fn value_stream(
        &self,
        initially_pending: bool,
    ) -> ValueStream<'v, T> {
        self.parent.value_stream(initially_pending)
    }
}

// Setter
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

    pub fn get(&self) -> T {
        self.parent.get()
    }
    pub fn set(
        &mut self,
        value: T,
    ) -> bool {
        self.parent.set(value)
    }

    pub fn observer(
        &self,
        initially_pending: bool,
    ) -> Observer<'v, T> {
        self.parent.observer(initially_pending)
    }
    pub fn changed_stream(
        &self,
        initially_pending: bool,
    ) -> ChangedStream<'v, T> {
        self.parent.changed_stream(initially_pending)
    }
    pub fn value_stream(
        &self,
        initially_pending: bool,
    ) -> ValueStream<'v, T> {
        self.parent.value_stream(initially_pending)
    }
}

// Observer
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

    pub fn changed(&mut self) -> ObserverChanged<'_, 'v, T> {
        ObserverChanged::new(self)
    }
}

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

    pub fn value(&self) -> &T {
        &self.pending_value
    }
    pub fn commit(self) {
        self.parent.last_seen_value.replace(self.pending_value);
    }
}

// ObserverChanged
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
    pub fn new(parent: &'r mut Observer<'v, T>) -> Self {
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
// yields () on value change
// multiple changes between polls will be compacted to single yield
// if value goes back - won't yield
// a little slower than ObserverChanged because of additional clone
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
// yields latest value
// multiple changes between polls will be compacted to single (last) yield
// if value goes back - won't yield
// a little slower than ObserverChanged because of additional clone
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
