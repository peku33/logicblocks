use super::types::{Class, TimeValue, Type};
use atomic_refcell::{AtomicRefCell, AtomicRefMut};
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
    items_receiver: AtomicRefCell<mpsc::UnboundedReceiver<TimeValue>>,
}
impl SinkBase {
    pub fn new(class: Class) -> Self {
        let (items_sender, items_receiver) = mpsc::unbounded::<TimeValue>();
        let items_receiver = AtomicRefCell::new(items_receiver);

        Self {
            class,
            items_sender,
            items_receiver,
        }
    }
    pub fn typed_ref<T: Type>(&self) -> Option<SinkTypedRef<'_, T>> {
        if self.class != T::class() {
            return None;
        }

        let typed_ref = SinkTypedRef::new(self);
        Some(typed_ref)
    }

    pub fn items_receiver_borrow_mut(
        &self
    ) -> AtomicRefMut<'_, mpsc::UnboundedReceiver<TimeValue>> {
        self.items_receiver.borrow_mut()
    }
}
