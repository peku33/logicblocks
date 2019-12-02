use futures::pin_mut;
use futures::stream::Stream;
use futures::task::{Context, Poll, Waker};
use std::cell::RefCell;
use std::collections::LinkedList;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, Weak};

struct Common<T: Clone + Send + 'static> {
    receivers: LinkedList<Weak<Mutex<ReceiverInner<T>>>>,
}
impl<T: Clone + Send + 'static> Common<T> {
    fn new() -> Self {
        return Self {
            receivers: LinkedList::new(),
        };
    }
}

pub struct Sender<T: Clone + Send + 'static> {
    common: Rc<RefCell<Common<T>>>,
}
impl<T: Clone + Send + 'static> Sender<T> {
    fn new(common: Rc<RefCell<Common<T>>>) -> Self {
        return Self { common };
    }
    pub fn send(
        &self,
        item: T,
    ) -> () {
        self.common
            .borrow_mut()
            .receivers
            .drain_filter(|receiver_inner| {
                let receiver_inner = match receiver_inner.upgrade() {
                    Some(receiver_inner) => receiver_inner,
                    None => return true,
                };
                let mut receiver_inner = receiver_inner.lock().unwrap();
                receiver_inner.push(item.clone());
                return false;
            });
    }
}

pub struct ReceiverFactory<T: Clone + Send + 'static> {
    common: Rc<RefCell<Common<T>>>,
}
impl<T: Clone + Send + 'static> ReceiverFactory<T> {
    fn new(common: Rc<RefCell<Common<T>>>) -> Self {
        return Self { common };
    }
    pub fn receiver(&self) -> Receiver<T> {
        let receiver_inner_arc_mutex = Arc::new(Mutex::new(ReceiverInner::new()));
        self.common
            .borrow_mut()
            .receivers
            .push_back(Arc::downgrade(&receiver_inner_arc_mutex));
        return Receiver::new(receiver_inner_arc_mutex);
    }
}

struct ReceiverInner<T: Clone + Send + 'static> {
    queue: LinkedList<T>,
    waker: Option<Waker>,
}
impl<T: Clone + Send + 'static> ReceiverInner<T> {
    fn new() -> Self {
        return Self {
            queue: LinkedList::new(),
            waker: None,
        };
    }
    fn push(
        &mut self,
        item: T,
    ) -> () {
        self.queue.push_back(item);
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}
impl<T: Clone + Send + 'static> Stream for ReceiverInner<T> {
    type Item = T;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let self_ = self.get_mut();
        if let Some(first) = self_.queue.pop_front() {
            return Poll::Ready(Some(first));
        } else {
            self_.waker.replace(cx.waker().clone());
            return Poll::Pending;
        }
    }
}

pub struct Receiver<T: Clone + Send + 'static> {
    inner: Arc<Mutex<ReceiverInner<T>>>,
}
impl<T: Clone + Send + 'static> Receiver<T> {
    fn new(inner: Arc<Mutex<ReceiverInner<T>>>) -> Self {
        return Self { inner };
    }
}
impl<T: Clone + Send + 'static> Stream for Receiver<T> {
    type Item = T;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let mut receiver_inner = self.inner.lock().unwrap();
        let receiver_inner = &mut *receiver_inner;
        pin_mut!(receiver_inner);
        return receiver_inner.poll_next(cx);
    }
}

pub fn channel<T: Clone + Send + 'static>() -> (Sender<T>, ReceiverFactory<T>) {
    let common = Rc::new(RefCell::new(Common::new()));
    let sender = Sender::new(common.clone());
    let receiver_factory = ReceiverFactory::new(common);
    return (sender, receiver_factory);
}
