use crossbeam::queue::SegQueue;
use futures::stream::{FusedStream, Stream};
use futures::task::AtomicWaker;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};

struct Common<T: Clone + Send + 'static> {
    receivers: HashSet<ReceiverInnerPointer<T>>,
}
impl<T: Clone + Send + 'static> Common<T> {
    fn new() -> Self {
        Self {
            receivers: HashSet::new(),
        }
    }
}

#[derive(Clone)]
pub struct Sender<T: Clone + Send + 'static> {
    common: Arc<RwLock<Common<T>>>,
}
impl<T: Clone + Send + 'static> Sender<T> {
    pub fn new() -> Self {
        Self::new_from_common(Arc::new(RwLock::new(Common::new())))
    }
    fn new_from_common(common: Arc<RwLock<Common<T>>>) -> Self {
        Self { common }
    }

    pub fn send(
        &self,
        value: T,
    ) {
        self.common
            .read()
            .unwrap()
            .receivers
            .iter()
            .for_each(|receiver| {
                let receiver = unsafe { receiver.0.as_ref() };
                receiver.queue.push(value.clone());
                receiver.waker.wake();
            });
    }

    pub fn sender(&self) -> Sender<T> {
        Sender::new_from_common(self.common.clone())
    }
    pub fn receiver_factory(&self) -> ReceiverFactory<T> {
        ReceiverFactory::new_from_common(self.common.clone())
    }
    pub fn receiver(&self) -> Receiver<T> {
        Receiver::new_from_common(self.common.clone())
    }
}
impl<T: Clone + Send + 'static> fmt::Debug for Sender<T> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        f.pad("Sender { ... }")
    }
}

#[derive(Clone)]
pub struct ReceiverFactory<T: Clone + Send + 'static> {
    common: Arc<RwLock<Common<T>>>,
}
impl<T: Clone + Send + 'static> ReceiverFactory<T> {
    pub fn new() -> Self {
        Self::new_from_common(Arc::new(RwLock::new(Common::new())))
    }

    fn new_from_common(common: Arc<RwLock<Common<T>>>) -> Self {
        Self { common }
    }

    pub fn sender(&self) -> Sender<T> {
        Sender::new_from_common(self.common.clone())
    }
    pub fn receiver_factory(&self) -> ReceiverFactory<T> {
        ReceiverFactory::new_from_common(self.common.clone())
    }
    pub fn receiver(&self) -> Receiver<T> {
        Receiver::new_from_common(self.common.clone())
    }
}
impl<T: Clone + Send + 'static> fmt::Debug for ReceiverFactory<T> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        f.pad("ReceiverFactory { ... }")
    }
}

struct ReceiverInner<T: Clone + Send + 'static> {
    queue: SegQueue<T>,
    waker: AtomicWaker,
    pin: PhantomPinned,
}

pub struct Receiver<T: Clone + Send + 'static> {
    common: Arc<RwLock<Common<T>>>,
    receiver_inner: Pin<Box<ReceiverInner<T>>>,
}
impl<T: Clone + Send + 'static> Receiver<T> {
    pub fn new() -> Self {
        Self::new_from_common(Arc::new(RwLock::new(Common::new())))
    }

    fn new_from_common(common: Arc<RwLock<Common<T>>>) -> Self {
        let receiver_inner = Box::pin(ReceiverInner {
            queue: SegQueue::new(),
            waker: AtomicWaker::new(),
            pin: PhantomPinned,
        });
        let receiver_inner_pointer = ReceiverInnerPointer((&*receiver_inner).into());
        common
            .write()
            .unwrap()
            .receivers
            .insert(receiver_inner_pointer);
        Self {
            common,
            receiver_inner,
        }
    }

    pub fn sender(&self) -> Sender<T> {
        Sender::new_from_common(self.common.clone())
    }
    pub fn receiver_factory(&self) -> ReceiverFactory<T> {
        ReceiverFactory::new_from_common(self.common.clone())
    }
    pub fn receiver(&self) -> Receiver<T> {
        Receiver::new_from_common(self.common.clone())
    }
}
impl<T: Clone + Send + 'static> Stream for Receiver<T> {
    type Item = T;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        self.receiver_inner.waker.register(cx.waker());
        match self.receiver_inner.queue.pop().ok() {
            Some(item) => Poll::Ready(Some(item)),
            None => Poll::Pending,
        }
    }
}
impl<T: Clone + Send + 'static> FusedStream for Receiver<T> {
    fn is_terminated(&self) -> bool {
        false
    }
}
impl<T: Clone + Send + 'static> Drop for Receiver<T> {
    fn drop(&mut self) {
        let receiver_inner_pointer = ReceiverInnerPointer((&*self.receiver_inner).into());
        self.common
            .write()
            .unwrap()
            .receivers
            .remove(&receiver_inner_pointer);
    }
}
impl<T: Clone + Send + 'static> fmt::Debug for Receiver<T> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        f.pad("Receiver { ... }")
    }
}

struct ReceiverInnerPointer<T: Clone + Send + 'static>(NonNull<ReceiverInner<T>>);
unsafe impl<T: Clone + Send + 'static> Sync for ReceiverInnerPointer<T> {}
unsafe impl<T: Clone + Send + 'static> Send for ReceiverInnerPointer<T> {}
impl<T: Clone + Send + 'static> Hash for ReceiverInnerPointer<T> {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.0.hash(state)
    }
}
impl<T: Clone + Send + 'static> PartialEq for ReceiverInnerPointer<T> {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.0.eq(&other.0)
    }
}
impl<T: Clone + Send + 'static> Eq for ReceiverInnerPointer<T> {}
