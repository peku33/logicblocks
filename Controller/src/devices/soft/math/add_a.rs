use crate::{
    devices,
    signals::{self, signal, types::state::Value},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use itertools::chain;
use std::{any::type_name, borrow::Cow, iter, ops::Add};

#[derive(Debug)]
pub struct Configuration<L, R = L, O = <L as Add<R>>::Output>
where
    L: Value + Copy + Clone,
    R: Value + Copy + Clone,
    O: Value + Copy + Clone,
    L: Add<R, Output = O>,
{
    pub left_fixed: Option<L>,
    pub right_fixed: Option<R>,
}

#[derive(Debug)]
pub struct Device<L, R = L, O = <L as Add<R>>::Output>
where
    L: Value + Copy + Clone,
    R: Value + Copy + Clone,
    O: Value + Copy + Clone,
    L: Add<R, Output = O>,
{
    configuration: Configuration<L, R, O>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_left: Option<signal::state_target_last::Signal<L>>,
    signal_right: Option<signal::state_target_last::Signal<R>>,
    signal_output: signal::state_source::Signal<O>,
}
impl<L, R, O> Device<L, R, O>
where
    L: Value + Copy + Clone,
    R: Value + Copy + Clone,
    O: Value + Copy + Clone,
    L: Add<R, Output = O>,
{
    pub fn new(configuration: Configuration<L, R, O>) -> Self {
        let left_fixed = configuration.left_fixed.is_some();
        let right_fixed = configuration.right_fixed.is_some();

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_left: if !left_fixed {
                Some(signal::state_target_last::Signal::<L>::new())
            } else {
                None
            },
            signal_right: if !right_fixed {
                Some(signal::state_target_last::Signal::<R>::new())
            } else {
                None
            },
            signal_output: signal::state_source::Signal::<O>::new(None),
        }
    }

    fn calculate(
        left: L,
        right: R,
    ) -> O {
        left + right
    }
    fn calculate_optional(
        left: Option<L>,
        right: Option<R>,
    ) -> Option<O> {
        Some(Self::calculate(left?, right?))
    }

    fn signals_targets_changed(&self) {
        let left = self
            .signal_left
            .as_ref()
            .and_then(|signal_left| signal_left.take_last().value);
        let left = match self.configuration.left_fixed {
            Some(left_fixed) => Some(left_fixed),
            None => left,
        };

        let right = self
            .signal_right
            .as_ref()
            .and_then(|signal_right| signal_right.take_last().value);
        let right = match self.configuration.right_fixed {
            Some(right_fixed) => Some(right_fixed),
            None => right,
        };

        let output = Self::calculate_optional(left, right);

        if self.signal_output.set_one(output) {
            self.signals_sources_changed_waker.wake();
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.signals_targets_changed_waker
            .stream()
            .stream_take_until_exhausted(exit_flag)
            .for_each(async |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl<L, R, O> devices::Device for Device<L, R, O>
where
    L: Value + Copy + Clone,
    R: Value + Copy + Clone,
    O: Value + Copy + Clone,
    L: Add<R, Output = O>,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!(
            "soft/math/add_a<{}, {}, {}>",
            type_name::<L>(),
            type_name::<R>(),
            type_name::<O>(),
        ))
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
}

#[async_trait]
impl<L, R, O> Runnable for Device<L, R, O>
where
    L: Value + Copy + Clone,
    R: Value + Copy + Clone,
    O: Value + Copy + Clone,
    L: Add<R, Output = O>,
{
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalIdentifier {
    Left,
    Right,
    Output,
}
impl signals::Identifier for SignalIdentifier {}
impl<L, R, O> signals::Device for Device<L, R, O>
where
    L: Value + Copy + Clone,
    R: Value + Copy + Clone,
    O: Value + Copy + Clone,
    L: Add<R, Output = O>,
{
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        Some(&self.signals_targets_changed_waker)
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        Some(&self.signals_sources_changed_waker)
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        chain!(
            self.signal_left
                .as_ref()
                .map(|signal_left| { (SignalIdentifier::Left, signal_left as &dyn signal::Base,) }),
            self.signal_right.as_ref().map(|signal_right| {
                (SignalIdentifier::Right, signal_right as &dyn signal::Base)
            }),
            iter::once((
                SignalIdentifier::Output,
                &self.signal_output as &dyn signal::Base,
            )),
        )
        .collect::<signals::ByIdentifier<_>>()
    }
}
