use super::{
    super::{signal::RemoteBaseVariant, Device},
    connections_requested::Connections as ConnectionsRequested,
    connections_running::{
        Connections as ConnectionsRunning, Event as ConnectionsRunningEvent,
        State as ConnectionsRunningState,
    },
    DeviceIdSignalId,
};
use crate::{
    devices::Id as DeviceId,
    util::{
        async_flag,
        ready_chunks_dynamic::ReadyChunksDynamicExt,
        runtime::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::{
    future::FutureExt,
    pin_mut, select,
    stream::{SelectAll, StreamExt},
};
use std::collections::{HashMap, HashSet};

pub struct Exchanger<'d> {
    devices: HashMap<DeviceId, &'d dyn Device>,
    connections_running: ConnectionsRunning<'d>,
}
impl<'d> Exchanger<'d> {
    pub fn new(
        devices: HashMap<DeviceId, &'d dyn Device>,
        connections_requested: &ConnectionsRequested,
    ) -> Self {
        let connections_running =
            connections_requested_to_connections_running(&devices, connections_requested);

        Self {
            devices,
            connections_running,
        }
    }

    pub async fn run(&self) -> ! {
        let mut device_signal_sources_changed_wakers = self
            .devices
            .iter()
            .map(move |(device_id, device)| {
                (*device_id, device.signal_sources_changed_waker_receiver())
            })
            .collect::<HashMap<DeviceId, _>>();

        let device_signal_sources_changed_runner = device_signal_sources_changed_wakers
            .iter_mut()
            .map(move |(device_id, device_signal_sources_changed_waker)| {
                let device_id = *device_id;
                device_signal_sources_changed_waker
                    .by_ref()
                    .map(move |()| device_id)
            })
            .collect::<SelectAll<_>>()
            .ready_chunks_dynamic()
            .for_each(async move |device_ids| {
                let device_ids = device_ids.into_iter().collect();
                self.signal_sources_changed(Some(device_ids)).await
            });
        pin_mut!(device_signal_sources_changed_runner);

        self.signal_state_forward_all().await;
        self.signal_sources_changed(None).await;

        select! {
            _ = device_signal_sources_changed_runner => panic!("device_signal_sources_changed_runner yielded")
        }
    }

    async fn signal_state_forward_all(&self) {
        let mut target_device_ids: HashSet<DeviceId> = HashSet::new();

        for ((target_device_id_signal_id, state_target_remote_base), source) in
            self.connections_running.state().iter_targets()
        {
            let value = match source {
                Some((_, state_source_remote_base)) => state_source_remote_base.get_last(),
                None => None,
            };

            if state_target_remote_base.set(&[value]) {
                target_device_ids.insert(target_device_id_signal_id.device_id);
            }
        }

        for target_device_id in target_device_ids.into_iter() {
            self.devices
                .get(&target_device_id)
                .unwrap()
                .signal_targets_changed_wake();
        }
    }

    async fn signal_sources_changed(
        &self,
        device_ids: Option<HashSet<DeviceId>>, // None - all
    ) {
        let mut target_device_ids: HashSet<DeviceId> = HashSet::new();

        // State
        for ((source_device_id_signal_id, state_source_remote_base), targets) in
            self.connections_running.state().iter_sources()
        {
            if let Some(device_ids) = device_ids.as_ref() {
                if !device_ids.contains(&source_device_id_signal_id.device_id) {
                    continue;
                }
            }

            let pending = state_source_remote_base.take_pending();
            if pending.is_empty() {
                continue;
            }

            for (target_device_id_signal_id, state_target_remote_base) in targets {
                if state_target_remote_base.set(&pending) {
                    target_device_ids.insert(target_device_id_signal_id.device_id);
                }
            }
        }

        // Event
        for ((source_device_id_signal_id, state_source_remote_base), targets) in
            self.connections_running.event().iter_sources()
        {
            if let Some(device_ids) = device_ids.as_ref() {
                if !device_ids.contains(&source_device_id_signal_id.device_id) {
                    continue;
                }
            }

            let pending = state_source_remote_base.take_pending();
            if pending.is_empty() {
                continue;
            }

            for (target_device_id_signal_id, state_target_remote_base) in targets {
                if state_target_remote_base.push(&pending) {
                    target_device_ids.insert(target_device_id_signal_id.device_id);
                }
            }
        }

        // Notify devices about changes
        for target_device_id in target_device_ids.into_iter() {
            self.devices
                .get(&target_device_id)
                .unwrap()
                .signal_targets_changed_wake();
        }
    }
}
#[async_trait]
impl<'d> Runnable for Exchanger<'d> {
    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let run_future = self.run();
        pin_mut!(run_future);
        let mut run_future = run_future.fuse();

        select! {
            _ = run_future => panic!("run_future yielded"),
            () = exit_flag => {},
        }

        Exited
    }
}

fn connections_requested_to_connections_running<'d>(
    devices: &HashMap<DeviceId, &'d dyn Device>,
    connections_requested: &ConnectionsRequested,
) -> ConnectionsRunning<'d> {
    let mut connections_state = ConnectionsRunningState::new();
    let mut connections_event = ConnectionsRunningEvent::new();

    // Process sources / targets from devices
    for (device_id, device) in devices.iter() {
        for (signal_id, signal) in device.signals().iter() {
            let device_id_signal_id = DeviceIdSignalId {
                device_id: *device_id,
                signal_id: *signal_id,
            };

            let signal_remote_base = signal.as_remote_base();
            let signal_remote_base_variant = signal_remote_base.as_remote_base_variant();
            match signal_remote_base_variant {
                RemoteBaseVariant::StateSource(state_source_remote_base) => {
                    connections_state.insert_source(device_id_signal_id, state_source_remote_base)
                }
                RemoteBaseVariant::StateTarget(state_target_remote_base) => {
                    connections_state.insert_target(device_id_signal_id, state_target_remote_base)
                }
                RemoteBaseVariant::EventSource(event_source_remote_base) => {
                    connections_event.insert_source(device_id_signal_id, event_source_remote_base)
                }
                RemoteBaseVariant::EventTarget(event_target_remote_base) => {
                    connections_event.insert_target(device_id_signal_id, event_target_remote_base)
                }
            }
        }
    }

    // Process connections
    let mut connection_state_candidates =
        HashMap::<DeviceIdSignalId, HashSet<DeviceIdSignalId>>::new();
    let mut connection_event_candidates =
        HashMap::<DeviceIdSignalId, HashSet<DeviceIdSignalId>>::new();

    for (source_device_id_signal_id, target_device_id_signal_ids) in connections_requested.iter() {
        match (
            connections_state.source_details(source_device_id_signal_id),
            connections_event.source_details(source_device_id_signal_id),
        ) {
            (Some(state_source_remote_base), None) => {
                let source_type_id = state_source_remote_base.type_id();
                for target_device_id_signal_id in target_device_id_signal_ids {
                    match (
                        connections_state.target_details(target_device_id_signal_id),
                        connections_event.target_details(target_device_id_signal_id),
                    ) {
                        (Some(state_target_remote_base), None) => {
                            let target_type_id = state_target_remote_base.type_id();
                            if source_type_id == target_type_id {
                                let inserted = connection_state_candidates
                                    .entry(*source_device_id_signal_id)
                                    .or_default()
                                    .insert(*target_device_id_signal_id);
                                assert!(inserted);
                            } else {
                                log::warn!(
                                    "invalid state type {:?} ({:?}) -> {:?} ({:?})",
                                    source_device_id_signal_id,
                                    source_type_id,
                                    target_device_id_signal_id,
                                    target_type_id,
                                );
                            }
                        }
                        (None, Some(_)) => {
                            log::warn!(
                                "invalid state {:?} -> event {:?}",
                                source_device_id_signal_id,
                                target_device_id_signal_id
                            );
                        }
                        (None, None) => {
                            log::warn!("missing state target {:?}", target_device_id_signal_id);
                        }
                        (Some(_), Some(_)) => {
                            panic!(
                                "duplicated target state signal type: {:?}",
                                target_device_id_signal_id
                            );
                        }
                    }
                }
            }
            (None, Some(event_source_remote_base)) => {
                let source_type_id = event_source_remote_base.type_id();
                for target_device_id_signal_id in target_device_id_signal_ids {
                    match (
                        connections_state.target_details(target_device_id_signal_id),
                        connections_event.target_details(target_device_id_signal_id),
                    ) {
                        (None, Some(event_target_remote_base)) => {
                            let target_type_id = event_target_remote_base.type_id();
                            if source_type_id == target_type_id {
                                let inserted = connection_event_candidates
                                    .entry(*source_device_id_signal_id)
                                    .or_default()
                                    .insert(*target_device_id_signal_id);
                                assert!(inserted);
                            } else {
                                log::warn!(
                                    "invalid event type {:?} ({:?}) -> {:?} ({:?})",
                                    source_device_id_signal_id,
                                    source_type_id,
                                    target_device_id_signal_id,
                                    target_type_id,
                                );
                            }
                        }
                        (Some(_), None) => {
                            log::warn!(
                                "invalid event {:?} -> state {:?}",
                                source_device_id_signal_id,
                                target_device_id_signal_id
                            );
                        }
                        (None, None) => {
                            log::warn!("missing event target {:?}", target_device_id_signal_id);
                        }
                        (Some(_), Some(_)) => {
                            panic!(
                                "duplicated target event signal type: {:?}",
                                target_device_id_signal_id
                            );
                        }
                    }
                }
            }
            (None, None) => {
                log::warn!("missing source: {:?}", source_device_id_signal_id);
            }
            (Some(_), Some(_)) => {
                panic!(
                    "duplicated source signal type: {:?}",
                    source_device_id_signal_id
                );
            }
        }
    }

    // Process connection candidates to connections
    let mut connection_state_candidates_inverted =
        HashMap::<DeviceIdSignalId, HashSet<DeviceIdSignalId>>::new();
    for (source_device_id_signal_id, target_device_id_signal_ids) in connection_state_candidates {
        for target_device_id_signal_id in target_device_id_signal_ids {
            connection_state_candidates_inverted
                .entry(target_device_id_signal_id)
                .or_default()
                .insert(source_device_id_signal_id);
        }
    }
    let mut connection_state_candidates_inverted_pruned =
        HashMap::<DeviceIdSignalId, DeviceIdSignalId>::new();
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

        let source_device_id_signal_id = source_device_id_signal_ids.into_iter().next().unwrap();

        assert!(connection_state_candidates_inverted_pruned
            .insert(target_device_id_signal_id, source_device_id_signal_id)
            .is_none());
    }
    connections_state.set_connections(connection_state_candidates_inverted_pruned);

    connections_event.set_connections(connection_event_candidates);

    ConnectionsRunning::new(connections_state, connections_event)
}
