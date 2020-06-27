use crate::util::waker_stream;
use futures::{
    sink::Sink,
    stream::{FusedStream, Stream},
};
use parking_lot::Mutex;
use std::{
    cmp::max,
    convert::Infallible,
    ops::Deref,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct Property<T>
where
    T: Clone,
{
    local: Mutex<(Option<T>, usize)>, // (pending_value, version)
    device: Mutex<usize>,             // version
    waker: waker_stream::Sender,
}
impl<T> Property<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        let local = Mutex::new((None, 0));
        let device = Mutex::new(0);

        let waker = waker_stream::Sender::new();

        Self {
            local,
            device,
            waker,
        }
    }

    // User
    pub fn user_get_sink(&self) -> ValueSink<T> {
        ValueSink::new(self)
    }

    // Device
    pub fn device_get_stream(&self) -> impl Stream<Item = ()> + FusedStream {
        self.waker.receiver()
    }
    pub fn device_get_pending(&self) -> Option<Pending<T>> {
        let local = self.local.lock();
        let device = *self.device.lock();
        if local.1 > device {
            Some(Pending::new(
                self,
                local.0.as_ref().unwrap().clone(),
                local.1,
            ))
        } else {
            None
        }
    }
}

pub struct ValueSink<'p, T>
where
    T: Clone,
{
    property: &'p Property<T>,
    flush_pending: AtomicBool,
}
impl<'p, T> ValueSink<'p, T>
where
    T: Clone,
{
    fn new(property: &'p Property<T>) -> Self {
        Self {
            property,
            flush_pending: AtomicBool::new(false),
        }
    }

    pub fn set(
        &self,
        item: T,
    ) {
        let mut local = self.property.local.lock();
        local.0.replace(item);
        local.1 += 1;
        drop(local);

        self.property.waker.wake();
    }
}
impl<'p, T> Sink<T> for ValueSink<'p, T>
where
    T: Clone,
{
    type Error = Infallible;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: T,
    ) -> Result<(), Self::Error> {
        let mut local = self.property.local.lock();
        local.0.replace(item);
        local.1 += 1;
        drop(local);

        self.flush_pending.store(true, Ordering::Relaxed);

        Ok(())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        if self.flush_pending.swap(false, Ordering::Relaxed) {
            self.property.waker.wake();
        }

        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pub struct Pending<'p, T>
where
    T: Clone,
{
    parent: &'p Property<T>,

    value: T,
    version: usize,
}
impl<'p, T> Pending<'p, T>
where
    T: Clone,
{
    pub fn new(
        parent: &'p Property<T>,

        value: T,
        version: usize,
    ) -> Self {
        Self {
            parent,
            value,
            version,
        }
    }
    pub fn commit(self) {
        let mut device = self.parent.device.lock();
        *device = max(*device, self.version);
    }
}
impl<'p, T> Deref for Pending<'p, T>
where
    T: Clone,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
