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
    pub state: State<'d>,
    pub event: Event<'d>,
}
