use super::types::{Class, TimeValue, Type};
use crate::util::atomic_cell::{AtomicCell, AtomicCellLease};
use chrono::{DateTime, Utc};
use futures::channel::mpsc;
use std::marker::PhantomData;

// typed sink
#[derive(Debug)]
pub struct SinkTypedRef<'a, T: Type> {
    base: &'a SinkBase,
    type_: PhantomData<T>,
}
impl<'a, T: Type> SinkTypedRef<'a, T> {
    fn new(base: &'a SinkBase) -> Self {
        Self {
            base,
            type_: PhantomData,
        }
    }

    pub fn push(
        &self,
        time: DateTime<Utc>,
        value: Option<T>,
    ) {
        let value = T::into_value(value);
        let time_value = TimeValue { time, value };
        self.base.items_sender.unbounded_send(time_value).unwrap();
    }
}

// type-erased sink
#[derive(Debug)]
pub struct SinkBase {
    class: Class,

    items_sender: mpsc::UnboundedSender<TimeValue>,
    items_receiver: AtomicCell<mpsc::UnboundedReceiver<TimeValue>>,
}
impl SinkBase {
    pub fn new(class: Class) -> Self {
        let (items_sender, items_receiver) = mpsc::unbounded::<TimeValue>();
        let items_receiver = AtomicCell::new(items_receiver);

        Self {
            class,
            items_sender,
            items_receiver,
        }
    }
    pub fn typed_ref<T: Type>(&self) -> SinkTypedRef<'_, T> {
        assert!(self.class == T::class(), "class mismatch");
        SinkTypedRef::new(self)
    }

    pub fn items_receiver_lease(&self) -> AtomicCellLease<mpsc::UnboundedReceiver<TimeValue>> {
        self.items_receiver.lease()
    }
}
