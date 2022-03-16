use crate::util::waker_stream::mpsc_local;
use futures::stream::{FusedStream, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct InsChangedWaker {
    inner: mpsc_local::Signal,
}
impl InsChangedWaker {
    pub fn new() -> Self {
        let inner = mpsc_local::Signal::new();
        Self { inner }
    }

    pub fn wake(&self) {
        self.inner.wake();
    }

    pub fn remote(&self) -> InsChangedWakerRemote {
        InsChangedWakerRemote::new(self)
    }
}
#[derive(Debug)]
pub struct InsChangedWakerRemote<'a> {
    parent: &'a InsChangedWaker,
}
impl<'a> InsChangedWakerRemote<'a> {
    fn new(parent: &'a InsChangedWaker) -> Self {
        Self { parent }
    }
    pub fn stream(
        &self,
        initially_pending: bool,
    ) -> InsChangedWakerRemoteStream {
        InsChangedWakerRemoteStream::new(self, initially_pending)
    }
}
#[derive(Debug)]
pub struct InsChangedWakerRemoteStream<'a> {
    inner: mpsc_local::Receiver<'a>,
}
impl<'a> InsChangedWakerRemoteStream<'a> {
    fn new(
        parent: &'a InsChangedWakerRemote,
        initially_pending: bool,
    ) -> Self {
        let inner = parent.parent.inner.receiver(initially_pending);
        Self { inner }
    }
}
impl<'a> Stream for InsChangedWakerRemoteStream<'a> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let inner = unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner) };
        inner.poll_next(cx)
    }
}
impl<'a> FusedStream for InsChangedWakerRemoteStream<'a> {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

#[derive(Debug)]
pub struct OutsChangedWaker {
    inner: mpsc_local::Signal,
}
impl OutsChangedWaker {
    pub fn new() -> Self {
        let inner = mpsc_local::Signal::new();
        Self { inner }
    }

    pub fn stream(
        &self,
        initially_pending: bool,
    ) -> OutsChangedWakerStream {
        OutsChangedWakerStream::new(self, initially_pending)
    }

    pub fn remote(&self) -> OutsChangedWakerRemote {
        OutsChangedWakerRemote::new(self)
    }
}
#[derive(Debug)]
pub struct OutsChangedWakerStream<'a> {
    inner: mpsc_local::Receiver<'a>,
}
impl<'a> OutsChangedWakerStream<'a> {
    fn new(
        parent: &'a OutsChangedWaker,
        initially_pending: bool,
    ) -> Self {
        let inner = parent.inner.receiver(initially_pending);
        Self { inner }
    }
}
impl<'a> Stream for OutsChangedWakerStream<'a> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let inner = unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner) };
        inner.poll_next(cx)
    }
}
impl<'a> FusedStream for OutsChangedWakerStream<'a> {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}
#[derive(Debug)]
pub struct OutsChangedWakerRemote<'a> {
    inner: mpsc_local::Sender<'a>,
}
impl<'a> OutsChangedWakerRemote<'a> {
    fn new(parent: &'a OutsChangedWaker) -> Self {
        let inner = parent.inner.sender();
        Self { inner }
    }
    pub fn wake(&self) {
        self.inner.wake();
    }
}
