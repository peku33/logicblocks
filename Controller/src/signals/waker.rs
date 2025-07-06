use crate::util::async_waker::mpsc;
use futures::stream::{FusedStream, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct TargetsChangedWaker {
    inner: mpsc::Signal,
}
impl TargetsChangedWaker {
    pub fn new() -> Self {
        let inner = mpsc::Signal::new();
        Self { inner }
    }

    pub fn stream(&self) -> TargetsChangedWakerStream<'_> {
        TargetsChangedWakerStream::new(self)
    }

    pub(super) fn remote(&self) -> TargetsChangedWakerRemote<'_> {
        TargetsChangedWakerRemote::new(self)
    }
}
#[derive(Debug)]
pub struct TargetsChangedWakerStream<'a> {
    inner: mpsc::Receiver<'a>,
}
impl<'a> TargetsChangedWakerStream<'a> {
    fn new(parent: &'a TargetsChangedWaker) -> Self {
        let inner = parent.inner.receiver();
        Self { inner }
    }
}
impl Stream for TargetsChangedWakerStream<'_> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner) }.poll_next(cx)
    }
}
impl FusedStream for TargetsChangedWakerStream<'_> {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}
#[derive(Debug)]
pub struct TargetsChangedWakerRemote<'a> {
    inner: mpsc::Sender<'a>,
}
impl<'a> TargetsChangedWakerRemote<'a> {
    fn new(parent: &'a TargetsChangedWaker) -> Self {
        let inner = parent.inner.sender();
        Self { inner }
    }
    pub fn wake(&self) {
        self.inner.wake();
    }
}

#[derive(Debug)]
pub struct SourcesChangedWaker {
    inner: mpsc::Signal,
}
impl SourcesChangedWaker {
    pub fn new() -> Self {
        let inner = mpsc::Signal::new();
        Self { inner }
    }

    pub fn wake(&self) {
        self.inner.wake();
    }

    pub(super) fn remote(&self) -> SourcesChangedWakerRemote<'_> {
        SourcesChangedWakerRemote::new(self)
    }
}
#[derive(Debug)]
pub struct SourcesChangedWakerRemote<'a> {
    parent: &'a SourcesChangedWaker,
}
impl<'a> SourcesChangedWakerRemote<'a> {
    fn new(parent: &'a SourcesChangedWaker) -> Self {
        Self { parent }
    }
    pub fn stream(&'_ self) -> SourcesChangedWakerRemoteStream<'_> {
        SourcesChangedWakerRemoteStream::new(self)
    }
}
#[derive(Debug)]
pub struct SourcesChangedWakerRemoteStream<'a> {
    inner: mpsc::Receiver<'a>,
}
impl<'a> SourcesChangedWakerRemoteStream<'a> {
    fn new(parent: &'a SourcesChangedWakerRemote) -> Self {
        let inner = parent.parent.inner.receiver();
        Self { inner }
    }
}
impl Stream for SourcesChangedWakerRemoteStream<'_> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner) }.poll_next(cx)
    }
}
impl FusedStream for SourcesChangedWakerRemoteStream<'_> {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}
