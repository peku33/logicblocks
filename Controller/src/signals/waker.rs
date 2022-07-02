use crate::util::waker_stream::mpsc;
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

    pub fn stream(&self) -> TargetsChangedWakerStream {
        TargetsChangedWakerStream::new(self)
    }

    pub(super) fn remote(&self) -> TargetsChangedWakerRemote {
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
impl<'a> Stream for TargetsChangedWakerStream<'a> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let inner = unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner) };
        inner.poll_next(cx)
    }
}
impl<'a> FusedStream for TargetsChangedWakerStream<'a> {
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

    pub(super) fn remote(&self) -> SourcesChangedWakerRemote {
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
    pub fn stream(&self) -> SourcesChangedWakerRemoteStream {
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
impl<'a> Stream for SourcesChangedWakerRemoteStream<'a> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let inner = unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner) };
        inner.poll_next(cx)
    }
}
impl<'a> FusedStream for SourcesChangedWakerRemoteStream<'a> {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}
