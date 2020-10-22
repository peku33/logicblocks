use super::{
    super::super::signals::signal::{
        EventSourceRemoteBase, EventTargetRemoteBase, StateSourceRemoteBase, StateTargetRemoteBase,
    },
    connections_map::{ManyFromMany, ManyFromOne},
    DeviceIdSignalId,
};
use std::{fmt, hash};

pub type State<'d> = ManyFromOne<
    DeviceIdSignalId,
    &'d dyn StateSourceRemoteBase,
    DeviceIdSignalId,
    &'d dyn StateTargetRemoteBase,
>;

pub type Event<'d> = ManyFromMany<
    DeviceIdSignalId,
    &'d dyn EventSourceRemoteBase,
    DeviceIdSignalId,
    &'d dyn EventTargetRemoteBase,
>;

pub struct Connections<'d>
where
    DeviceIdSignalId: hash::Hash + PartialEq + Eq + Copy + Clone + fmt::Debug,
{
    state: State<'d>,
    event: Event<'d>,
}
impl<'d> Connections<'d>
where
    DeviceIdSignalId: hash::Hash + PartialEq + Eq + Copy + Clone + fmt::Debug,
{
    pub fn new(
        state: State<'d>,
        event: Event<'d>,
    ) -> Self {
        Self { state, event }
    }
    pub fn state(&self) -> &State<'d> {
        &self.state
    }
    pub fn event(&self) -> &Event<'d> {
        &self.event
    }
}
