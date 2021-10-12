use crate::{
    devices,
    signals::{self, signal, types::event::Value},
    util::waker_stream,
};
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

#[derive(Debug)]
pub struct Configuration<V: Value + Clone> {
    pub value: V,
}

#[derive(Debug)]
pub struct Device<V: Value + Clone> {
    configuration: Configuration<V>,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_trigger: signal::event_target_queued::Signal<()>,
    signal_output: signal::event_source::Signal<V>,
}
impl<V: Value + Clone> Device<V> {
    pub fn new(configuration: Configuration<V>) -> Self {
        Self {
            configuration,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_trigger: signal::event_target_queued::Signal::<()>::new(),
            signal_output: signal::event_source::Signal::<V>::new(),
        }
    }
}
impl<V: Value + Clone> devices::Device for Device<V> {
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/value/trigger_a<{}>", type_name::<V>()))
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
}
impl<V: Value + Clone> signals::Device for Device<V> {
    fn signal_targets_changed_wake(&self) {
        let values = self
            .signal_trigger
            .take_pending()
            .into_vec()
            .into_iter()
            .map(|()| self.configuration.value.clone())
            .collect::<Box<[_]>>();

        if self.signal_output.push_many(values) {
            self.signal_sources_changed_waker.wake();
        }
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_trigger as &dyn signal::Base,
            1 => &self.signal_output as &dyn signal::Base,
        }
    }
}
