use crate::util::async_waker::mpsc;
use futures::stream::{FusedStream, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct InsChangedWaker {
    inner: mpsc::Signal,
}
impl InsChangedWaker {
    pub fn new() -> Self {
        let inner = mpsc::Signal::new();
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
    pub fn stream(&self) -> InsChangedWakerRemoteStream {
        InsChangedWakerRemoteStream::new(self)
    }
}
#[derive(Debug)]
pub struct InsChangedWakerRemoteStream<'a> {
    inner: mpsc::Receiver<'a>,
}
impl<'a> InsChangedWakerRemoteStream<'a> {
    fn new(parent: &'a InsChangedWakerRemote) -> Self {
        let inner = parent.parent.inner.receiver();
        Self { inner }
    }
}
impl Stream for InsChangedWakerRemoteStream<'_> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner) }.poll_next(cx)
    }
}
impl FusedStream for InsChangedWakerRemoteStream<'_> {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

#[derive(Debug)]
pub struct OutsChangedWaker {
    inner: mpsc::Signal,
}
impl OutsChangedWaker {
    pub fn new() -> Self {
        let inner = mpsc::Signal::new();
        Self { inner }
    }

    pub fn stream(&self) -> OutsChangedWakerStream {
        OutsChangedWakerStream::new(self)
    }

    pub fn remote(&self) -> OutsChangedWakerRemote {
        OutsChangedWakerRemote::new(self)
    }
}
#[derive(Debug)]
pub struct OutsChangedWakerStream<'a> {
    inner: mpsc::Receiver<'a>,
}
impl<'a> OutsChangedWakerStream<'a> {
    fn new(parent: &'a OutsChangedWaker) -> Self {
        let inner = parent.inner.receiver();
        Self { inner }
    }
}
impl Stream for OutsChangedWakerStream<'_> {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner) }.poll_next(cx)
    }
}
impl FusedStream for OutsChangedWakerStream<'_> {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}
#[derive(Debug)]
pub struct OutsChangedWakerRemote<'a> {
    inner: mpsc::Sender<'a>,
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
