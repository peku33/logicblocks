use super::{
    device::SignalId,
    signal::{
        event_source::RemoteBase as EventSourceRemoteBase,
        event_target::RemoteBase as EventTargetRemoteBase,
        state_source::RemoteBase as StateSourceRemoteBase,
        state_target::RemoteBase as StateTargetRemoteBase, SignalRemoteBase,
    },
};
use crate::util::select_all_empty::SelectAllEmptyFutureInfinite;
use futures::{future::pending, pin_mut, select, stream::StreamExt};
use std::{
    collections::{HashMap, HashSet},
    fmt, hash,
};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct DeviceIdSignalId<DeviceId>
where
    DeviceId: Copy + PartialEq + Eq + hash::Hash + fmt::Debug,
{
    device_id: DeviceId,
    signal_id: SignalId,
}
impl<DeviceId> DeviceIdSignalId<DeviceId>
where
    DeviceId: Copy + PartialEq + Eq + hash::Hash + fmt::Debug,
{
    pub fn new(
        device_id: DeviceId,
        signal_id: SignalId,
    ) -> Self {
        Self {
            device_id,
            signal_id,
        }
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }
    pub fn signal_id(&self) -> SignalId {
        self.signal_id
    }
}

pub type Connections<DeviceId> =
    HashMap<DeviceIdSignalId<DeviceId>, HashSet<DeviceIdSignalId<DeviceId>>>;

pub struct SignalsRunner<DeviceId>
where
    DeviceId: Copy + PartialEq + Eq + hash::Hash + fmt::Debug,
{
    connections_state: ManyFromOne<
        DeviceIdSignalId<DeviceId>,
        Box<dyn StateSourceRemoteBase>,
        DeviceIdSignalId<DeviceId>,
        Box<dyn StateTargetRemoteBase>,
    >,
    connections_event: ManyFromMany<
        DeviceIdSignalId<DeviceId>,
        Box<dyn EventSourceRemoteBase>,
        DeviceIdSignalId<DeviceId>,
        Box<dyn EventTargetRemoteBase>,
    >,
}
impl<DeviceId> SignalsRunner<DeviceId>
where
    DeviceId: Copy + PartialEq + Eq + hash::Hash + fmt::Debug,
{
    // log::* somehow bumps it up
    #[allow(clippy::cognitive_complexity)]
    pub fn new(
        signals_remote: HashMap<DeviceIdSignalId<DeviceId>, SignalRemoteBase>,
        connections: &Connections<DeviceId>,
    ) -> Self {
        log::trace!("initializing SignalsRunner");

        log::trace!("signals_remote keys: {:?}", signals_remote.keys(),);
        log::trace!("connections: {:?}", connections);

        let mut connections_state = ManyFromOne::new();
        let mut connections_event = ManyFromMany::new();

        // Process devices
        for (device_id_signal_id, signal_remote) in signals_remote {
            match signal_remote {
                SignalRemoteBase::StateSource(state_source_remote_base) => {
                    connections_state.insert_source(device_id_signal_id, state_source_remote_base);
                }
                SignalRemoteBase::StateTarget(state_target_remote_base) => {
                    connections_state.insert_target(device_id_signal_id, state_target_remote_base);
                }
                SignalRemoteBase::EventSource(event_source_remote_base) => {
                    connections_event.insert_source(device_id_signal_id, event_source_remote_base);
                }
                SignalRemoteBase::EventTarget(event_target_remote_base) => {
                    connections_event.insert_target(device_id_signal_id, event_target_remote_base);
                }
            }
        }

        log::trace!("connections_state: {:?}", connections_state);
        log::trace!("connections_event: {:?}", connections_event);

        // Process connections
        let mut connection_state_candidates =
            HashMap::<DeviceIdSignalId<DeviceId>, HashSet<DeviceIdSignalId<DeviceId>>>::new();
        let mut connection_event_candidates =
            HashMap::<DeviceIdSignalId<DeviceId>, HashSet<DeviceIdSignalId<DeviceId>>>::new();

        for (source_device_id_signal_id, target_device_id_signal_ids) in connections {
            log::trace!(
                "considering connection {:?} -> {:?}",
                source_device_id_signal_id,
                target_device_id_signal_ids
            );

            match (
                connections_state.get_source_details(source_device_id_signal_id),
                connections_event.get_source_details(source_device_id_signal_id),
            ) {
                (Some(state_source_signal_remote), None) => {
                    let state_source_signal_remote_type_id = state_source_signal_remote.type_id();
                    for target_device_id_signal_id in target_device_id_signal_ids {
                        let state_target_signal_remote = match (
                            connections_state.get_target_details(target_device_id_signal_id),
                            connections_event.get_target_details(target_device_id_signal_id),
                        ) {
                            (Some(state_target_signal_remote), None) => state_target_signal_remote,
                            (None, Some(_)) => {
                                log::warn!(
                                    "invalid connection class: {:?} (state) -> {:?} (event)",
                                    source_device_id_signal_id,
                                    target_device_id_signal_id
                                );
                                continue;
                            }
                            (None, None) => {
                                log::warn!("missing target: {:?}", target_device_id_signal_id);
                                continue;
                            }
                            _ => panic!("duplicated signal type: {:?}", target_device_id_signal_id),
                        };

                        let state_target_signal_remote_type_id =
                            state_target_signal_remote.type_id();

                        if state_source_signal_remote_type_id != state_target_signal_remote_type_id
                        {
                            log::warn!(
                                "invalid data type: {:?} ({:?}) -> {:?} ({:?})",
                                source_device_id_signal_id,
                                state_source_signal_remote_type_id,
                                target_device_id_signal_id,
                                state_target_signal_remote_type_id
                            );
                            continue;
                        }

                        let inserted = connection_state_candidates
                            .entry(*source_device_id_signal_id)
                            .or_default()
                            .insert(*target_device_id_signal_id);
                        assert!(inserted);
                    }
                }
                (None, Some(event_source_signal_remote)) => {
                    let event_source_signal_remote_type_id = event_source_signal_remote.type_id();
                    for target_device_id_signal_id in target_device_id_signal_ids {
                        let event_target_signal_remote = match (
                            connections_event.get_target_details(target_device_id_signal_id),
                            connections_event.get_target_details(target_device_id_signal_id),
                        ) {
                            (Some(_), None) => {
                                log::warn!(
                                    "invalid connection class: {:?} (event) -> {:?} (state)",
                                    source_device_id_signal_id,
                                    target_device_id_signal_id
                                );
                                continue;
                            }
                            (None, Some(event_target_signal_remote)) => event_target_signal_remote,
                            (None, None) => {
                                log::warn!("missing target: {:?}", target_device_id_signal_id);
                                continue;
                            }
                            _ => panic!("duplicated signal type: {:?}", target_device_id_signal_id),
                        };

                        let event_target_signal_remote_type_id =
                            event_target_signal_remote.type_id();

                        if event_source_signal_remote_type_id != event_target_signal_remote_type_id
                        {
                            log::warn!(
                                "invalid data type: {:?} ({:?}) -> {:?} ({:?})",
                                source_device_id_signal_id,
                                event_source_signal_remote_type_id,
                                target_device_id_signal_id,
                                event_target_signal_remote_type_id
                            );
                            continue;
                        }

                        let inserted = connection_event_candidates
                            .entry(*source_device_id_signal_id)
                            .or_default()
                            .insert(*target_device_id_signal_id);
                        assert!(inserted);
                    }
                }
                (None, None) => {
                    log::warn!("missing source: {:?}", source_device_id_signal_id);
                    continue;
                }
                _ => panic!("duplicated signal type: {:?}", source_device_id_signal_id),
            };
        }

        log::trace!(
            "connection_state_candidates: {:?}",
            connection_state_candidates
        );
        log::trace!(
            "connection_event_candidates: {:?}",
            connection_event_candidates
        );

        // Build inverted signal list for deduplication
        // target_device_id_signal_id -> source_device_id_signal_ids
        let mut connection_state_candidates_inverted =
            HashMap::<DeviceIdSignalId<DeviceId>, HashSet<DeviceIdSignalId<DeviceId>>>::new();
        for (source_device_id_signal_id, target_device_id_signal_ids) in connections {
            for target_device_id_signal_id in target_device_id_signal_ids {
                connection_state_candidates_inverted
                    .entry(*target_device_id_signal_id)
                    .or_default()
                    .insert(*source_device_id_signal_id);
            }
        }

        log::trace!(
            "connection_state_candidates_inverted: {:?}",
            connection_state_candidates_inverted
        );

        // Retain only state connections where target has single source
        // Drop others, displaying error
        let mut connection_state_candidates_inverted_pruned =
            HashMap::<DeviceIdSignalId<DeviceId>, DeviceIdSignalId<DeviceId>>::new();
        for (target_device_id_signal_id, source_device_id_signal_ids) in
            connection_state_candidates_inverted
        {
            if source_device_id_signal_ids.len() != 1 {
                log::warn!(
                    "dropping all multiple source for state target: {:?} ({:?})",
                    target_device_id_signal_id,
                    source_device_id_signal_ids
                );
                continue;
            }

            let source_device_id_signal_id =
                source_device_id_signal_ids.into_iter().next().unwrap();

            connection_state_candidates_inverted_pruned
                .insert(target_device_id_signal_id, source_device_id_signal_id)
                .unwrap_none();
        }

        log::trace!(
            "connection_state_candidates_inverted_pruned: {:?}",
            connection_state_candidates_inverted_pruned
        );

        // Apply connections
        connections_state.set_connections(connection_state_candidates_inverted_pruned);
        connections_event.set_connections(connection_event_candidates);

        log::trace!("initialized SignalsRunner");

        Self {
            connections_state,
            connections_event,
        }
    }

    pub async fn run(&self) -> ! {
        log::trace!("run called");
        // Prepare connections

        // State - prepare streams
        log::trace!("preparing state connections");
        let run_state_source_stream_aggregated = self
            .connections_state
            .iter_sources()
            .map(async move |(source, targets)| {
                let (source_device_id_signal_id, ref source_remote_base) = source;

                let targets = targets.collect::<Vec<_>>();
                let targets = &targets;

                log::trace!(
                    "state stream: {:?} -> {:?}",
                    source_device_id_signal_id,
                    targets
                );

                source_remote_base
                    .get_stream()
                    .for_each(async move |()| {
                        let value = source_remote_base.get();

                        log::trace!(
                            "new value from {:?}: {:?}",
                            source_device_id_signal_id,
                            value
                        );

                        for target in targets.iter() {
                            let (target_device_id_signal_id, ref target_remote_base) = target;

                            log::trace!("forwarding to {:?}", target_device_id_signal_id);

                            target_remote_base.set_unwrap(value.clone());
                        }
                    })
                    .await;

                log::debug!("get_stream {:?} yielded", source_device_id_signal_id);

                pending::<()>().await;
                panic!("pending yielded");
            })
            .collect::<SelectAllEmptyFutureInfinite<_>>();
        pin_mut!(run_state_source_stream_aggregated);

        // State - forward initial values
        log::trace!("preparing state initial values");
        for (target, source) in self.connections_state.iter_targets() {
            let (target_device_id_signal_id, ref target_remote_base) = target;
            match source {
                Some((source_device_id_signal_id, ref source_remote_base)) => {
                    let value = source_remote_base.get();

                    log::trace!(
                        "target {:?} is being initialized from {:?} with value {:?}",
                        target_device_id_signal_id,
                        source_device_id_signal_id,
                        value,
                    );

                    target_remote_base.set_unwrap(value);
                }
                None => {
                    log::trace!("target {:?} is not initialized", target_device_id_signal_id);

                    target_remote_base.set_none();
                }
            }
        }

        // Event - prepare streams
        log::trace!("preparing event connections");
        let run_event_source_stream_aggregated = self
            .connections_event
            .iter_sources()
            .map(async move |(source, targets)| {
                let (source_device_id_signal_id, ref source_remote_base) = source;

                let targets = targets.collect::<Vec<_>>();
                let targets = &targets;

                log::trace!(
                    "event stream: {:?} -> {:?}",
                    source_device_id_signal_id,
                    targets
                );

                source_remote_base
                    .get_stream()
                    .for_each(async move |()| {
                        let pending_values = source_remote_base.take().into_vec();

                        log::trace!(
                            "new values from {:?}: {:?}",
                            source_device_id_signal_id,
                            pending_values
                        );

                        for pending_value in pending_values {
                            for target in targets.iter() {
                                let (target_device_id_signal_id, ref target_remote_base) = target;

                                log::trace!("forwarding to {:?}", target_device_id_signal_id);

                                target_remote_base.push_unwrap(pending_value.clone());
                            }
                        }
                    })
                    .await;

                log::debug!("get_stream {:?} exited", source_device_id_signal_id);

                pending::<()>().await;
                panic!("pending yielded");
            })
            .collect::<SelectAllEmptyFutureInfinite<_>>();
        pin_mut!(run_event_source_stream_aggregated);

        // State & Event - run streams
        log::trace!("running connections");
        select! {
            _ = run_state_source_stream_aggregated => panic!("run_state_source_stream_aggregated completed"),
            _ = run_event_source_stream_aggregated => panic!("run_event_source_stream_aggregated completed"),
        }

        // log::trace!("run() completed");
    }
}

// One key - multiple values
// One value - one key
#[derive(Debug)]
struct ManyFromOne<S, SD, T, TD>
where
    S: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    SD: fmt::Debug,
    T: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    TD: fmt::Debug,
{
    sources: HashMap<S, (SD, HashSet<T>)>,
    targets: HashMap<T, (TD, Option<S>)>,
}
impl<S, SD, T, TD> ManyFromOne<S, SD, T, TD>
where
    S: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    SD: fmt::Debug,
    T: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    TD: fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            targets: HashMap::new(),
        }
    }

    pub fn insert_source(
        &mut self,
        source: S,
        source_details: SD,
    ) {
        let duplicated = self
            .sources
            .insert(source, (source_details, HashSet::new()))
            .is_some();
        assert!(!duplicated);
    }
    pub fn insert_target(
        &mut self,
        target: T,
        target_details: TD,
    ) {
        let duplicated = self
            .targets
            .insert(target, (target_details, None))
            .is_some();
        assert!(!duplicated);
    }

    pub fn get_source_details(
        &self,
        source: &S,
    ) -> Option<&SD> {
        self.sources
            .get(source)
            .map(|(source_details, _)| source_details)
    }
    pub fn get_target_details(
        &self,
        target: &T,
    ) -> Option<&TD> {
        self.targets
            .get(target)
            .map(|(target_details, _)| target_details)
    }

    pub fn set_connections(
        &mut self,
        connections_inverted: HashMap<T, S>,
    ) {
        self.sources
            .values_mut()
            .for_each(|(_, targets)| targets.clear());
        self.targets
            .values_mut()
            .for_each(|(_, source)| *source = None);

        for (target, source) in connections_inverted {
            let inserted = self.sources.get_mut(&source).unwrap().1.insert(target);
            assert!(inserted);

            let duplicated = self
                .targets
                .get_mut(&target)
                .unwrap()
                .1
                .replace(source)
                .is_some();
            assert!(!duplicated)
        }
    }

    pub fn iter_sources(
        &self
    ) -> impl Iterator<Item = ((&S, &SD), impl Iterator<Item = (&T, &TD)>)> {
        self.sources
            .iter()
            .map(move |(source, (source_details, targets))| {
                (
                    (source, source_details),
                    targets
                        .iter()
                        .map(move |target| (target, &self.targets.get(target).unwrap().0)),
                )
            })
    }
    pub fn iter_targets(&self) -> impl Iterator<Item = ((&T, &TD), Option<(&S, &SD)>)> {
        self.targets
            .iter()
            .map(move |(target, (target_details, source))| {
                (
                    (target, target_details),
                    source
                        .as_ref()
                        .map(move |source| (source, &self.sources.get(source).unwrap().0)),
                )
            })
    }
}

#[derive(Debug)]
struct ManyFromMany<S, SD, T, TD>
where
    S: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    SD: fmt::Debug,
    T: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    TD: fmt::Debug,
{
    sources: HashMap<S, (SD, HashSet<T>)>,
    targets: HashMap<T, (TD, HashSet<S>)>,
}
impl<S, SD, T, TD> ManyFromMany<S, SD, T, TD>
where
    S: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    SD: fmt::Debug,
    T: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    TD: fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            targets: HashMap::new(),
        }
    }

    pub fn insert_source(
        &mut self,
        source: S,
        source_details: SD,
    ) {
        let duplicated = self
            .sources
            .insert(source, (source_details, HashSet::new()))
            .is_some();
        assert!(!duplicated);
    }
    pub fn insert_target(
        &mut self,
        target: T,
        target_details: TD,
    ) {
        let duplicated = self
            .targets
            .insert(target, (target_details, HashSet::new()))
            .is_some();
        assert!(!duplicated);
    }

    pub fn set_connections(
        &mut self,
        connections: HashMap<S, HashSet<T>>,
    ) {
        self.sources
            .values_mut()
            .for_each(|(_, targets)| targets.clear());
        self.targets
            .values_mut()
            .for_each(|(_, sources)| sources.clear());

        for (source, targets) in connections {
            self.sources
                .get_mut(&source)
                .unwrap()
                .1
                .extend(targets.iter().copied());
            for target in targets {
                let inserted = self.targets.get_mut(&target).unwrap().1.insert(source);
                assert!(inserted);
            }
        }
    }

    pub fn get_source_details(
        &self,
        source: &S,
    ) -> Option<&SD> {
        self.sources
            .get(source)
            .map(|(source_details, _)| source_details)
    }
    pub fn get_target_details(
        &self,
        target: &T,
    ) -> Option<&TD> {
        self.targets
            .get(target)
            .map(|(target_details, _)| target_details)
    }

    pub fn iter_sources(
        &self
    ) -> impl Iterator<Item = ((&S, &SD), impl Iterator<Item = (&T, &TD)>)> {
        self.sources
            .iter()
            .map(move |(source, (source_details, targets))| {
                (
                    (source, source_details),
                    targets
                        .iter()
                        .map(move |target| (target, &self.targets.get(target).unwrap().0)),
                )
            })
    }
}
