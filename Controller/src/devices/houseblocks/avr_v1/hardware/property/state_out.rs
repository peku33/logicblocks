use crate::util::waker_stream;
use futures::{
    sink::Sink,
    stream::{FusedStream, Stream},
};
use parking_lot::Mutex;
use std::{
    convert::Infallible,
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct Property<T>
where
    T: Clone + PartialEq,
{
    local_device_value: Mutex<(T, Option<T>)>,
    waker: waker_stream::Sender,
}
impl<T> Property<T>
where
    T: Clone + PartialEq,
{
    pub fn new(initial: T) -> Self {
        let local_device_value = Mutex::new((initial, None));
        let waker = waker_stream::Sender::new();

        Self {
            local_device_value,
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
        let local_device_value = self.local_device_value.lock();
        if !local_device_value.1.contains(&local_device_value.0) {
            Some(Pending::new(self, local_device_value.0.clone()))
        } else {
            None
        }
    }
}

pub struct ValueSink<'p, T>
where
    T: Clone + PartialEq,
{
    property: &'p Property<T>,
}
impl<'p, T> ValueSink<'p, T>
where
    T: Clone + PartialEq,
{
    fn new(property: &'p Property<T>) -> Self {
        Self { property }
    }

    pub fn set(
        &self,
        item: T,
    ) {
        let mut local_device_value = self.property.local_device_value.lock();
        if local_device_value.0 == item {
            return;
        }
        local_device_value.0 = item;
        drop(local_device_value);

        self.property.waker.wake();
    }
}
impl<'p, T> Sink<T> for ValueSink<'p, T>
where
    T: Clone + PartialEq,
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
        let mut local_device_value = self.property.local_device_value.lock();
        if local_device_value.0 == item {
            return Ok(());
        }
        local_device_value.0 = item;
        drop(local_device_value);

        Ok(())
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        self.property.waker.wake();

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
    T: Clone + PartialEq,
{
    parent: &'p Property<T>,
    value: T,
}
impl<'p, T> Pending<'p, T>
where
    T: Clone + PartialEq,
{
    fn new(
        parent: &'p Property<T>,
        value: T,
    ) -> Self {
        Self { parent, value }
    }
    pub fn commit(self) {
        let mut local_device_value = self.parent.local_device_value.lock();
        local_device_value.1.replace(self.value);
    }
}
impl<'p, T> Deref for Pending<'p, T>
where
    T: Clone + PartialEq,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
