#![allow(clippy::type_complexity)]
use super::{
    DeviceBaseRef, IdentifierBaseWrapper,
    signal::{
        Base, EventSourceRemoteBase, EventTargetRemoteBase, RemoteBase, RemoteBaseVariant,
        StateSourceRemoteBase, StateTargetRemoteBase,
    },
    waker::{SourcesChangedWakerRemote, TargetsChangedWakerRemote},
};
use crate::{
    devices::Id as DeviceId,
    util::{
        async_ext::{
            ready_chunks_dynamic::ReadyChunksDynamicExt,
            select_all_or_pending::StreamSelectAllOrPending,
            stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        },
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use anyhow::{Context, Error, anyhow, bail, ensure};
use async_trait::async_trait;
use by_address::ByAddress;
use futures::stream::StreamExt;
use ouroboros::self_referencing;
use std::collections::{HashMap, HashSet};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DeviceIdSignalIdentifierBaseWrapper {
    device_id: DeviceId,
    signal_identifier_base_wrapper: IdentifierBaseWrapper,
}

impl DeviceIdSignalIdentifierBaseWrapper {
    pub fn new(
        device_id: DeviceId,
        signal_identifier_base_wrapper: IdentifierBaseWrapper,
    ) -> Self {
        Self {
            device_id,
            signal_identifier_base_wrapper,
        }
    }
}

pub type ConnectionRequested = (
    DeviceIdSignalIdentifierBaseWrapper,
    DeviceIdSignalIdentifierBaseWrapper,
);

#[self_referencing]
#[derive(Debug)]
struct ExchangerInner<'d> {
    parent: ExchangerInnerParent<'d>,

    #[borrows(parent)]
    #[covariant]
    child: ExchangerInnerChild<'this, 'd>,
}

#[derive(Debug)]
struct ExchangerInnerParent<'d> {
    device_contexts: HashMap<
        DeviceId,
        (
            DeviceBaseRef<'d>,
            Option<TargetsChangedWakerRemote<'d>>,
            Option<SourcesChangedWakerRemote<'d>>,
            HashMap<IdentifierBaseWrapper, &'d dyn RemoteBase>,
        ),
    >,
}

#[derive(Debug)]
struct ExchangerInnerChild<'p, 'd> {
    connections: HashMap<
        ByAddress<&'p SourcesChangedWakerRemote<'d>>, // source waker
        (
            // state
            HashMap<
                // all signals of parent
                ByAddress<&'d dyn StateSourceRemoteBase>, // source signal
                HashMap<
                    ByAddress<&'d dyn StateTargetRemoteBase>, // target signal
                    ByAddress<&'p TargetsChangedWakerRemote<'d>>, // target waker
                >,
            >,
            // event
            HashMap<
                // all signals of parent
                ByAddress<&'d dyn EventSourceRemoteBase>, // source signal
                HashMap<
                    ByAddress<&'d dyn EventTargetRemoteBase>, // target signal
                    ByAddress<&'p TargetsChangedWakerRemote<'d>>, // target waker
                >,
            >,
        ),
    >,
    state_targets_disconnected: HashMap<
        ByAddress<&'d dyn StateTargetRemoteBase>,     // signal
        ByAddress<&'p TargetsChangedWakerRemote<'d>>, // waker
    >,
}

#[derive(Debug)]
pub struct Exchanger<'d> {
    inner: ExchangerInner<'d>,
}
impl<'d> Exchanger<'d> {
    pub fn new(
        devices: &HashMap<DeviceId, DeviceBaseRef<'d>>,
        connections_requested: &[ConnectionRequested],
    ) -> Result<Self, Error> {
        let inner = new_inner(devices, connections_requested).context("new_inner")?;
        Ok(Self { inner })
    }

    async fn sources_to_targets_all_run(&self) {
        let mut targets_changed_waker_remotes =
            HashSet::<ByAddress<&TargetsChangedWakerRemote>>::new();

        // push none to all disconnected state targets
        let values_state_disconnected = vec![None].into_boxed_slice();
        for (state_target_remote_base, targets_changed_waker_remote) in
            self.inner.borrow_child().state_targets_disconnected.iter()
        {
            if state_target_remote_base.set(&values_state_disconnected) {
                targets_changed_waker_remotes.insert(*targets_changed_waker_remote);
            }
        }

        // forward all values from sources to targets
        for (connections_state, connections_event) in self.inner.borrow_child().connections.values()
        {
            // state signals
            for (state_source_remote_base, connection_targets) in connections_state.iter() {
                // forward pending values (including last)
                // if there is no pending value, use just the last value
                let mut values = state_source_remote_base.take_pending();
                if values.is_empty() {
                    values = vec![state_source_remote_base.peek_last()].into_boxed_slice();
                }

                for (state_target_remote_base, targets_changed_waker_remote) in
                    connection_targets.iter()
                {
                    if state_target_remote_base.set(&values) {
                        targets_changed_waker_remotes.insert(*targets_changed_waker_remote);
                    }
                }
            }

            // event signals
            for (event_source_remote_base, connection_targets) in connections_event.iter() {
                let values = event_source_remote_base.take_pending();
                if values.is_empty() {
                    continue;
                }

                for (event_target_remote_base, targets_changed_waker_remote) in
                    connection_targets.iter()
                {
                    if event_target_remote_base.push(&values) {
                        targets_changed_waker_remotes.insert(*targets_changed_waker_remote);
                    }
                }
            }
        }

        for targets_changed_waker_remote in targets_changed_waker_remotes {
            targets_changed_waker_remote.wake();
        }
    }

    async fn sources_to_targets_wakers_run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        let mut sources_changed_waker_remote_streams = self
            .inner
            .borrow_child()
            .connections
            .keys()
            .map(|sources_changed_waker_remote| {
                let sources_changed_waker_remote_stream = sources_changed_waker_remote.stream();

                (
                    sources_changed_waker_remote,
                    sources_changed_waker_remote_stream,
                )
            })
            .collect::<Box<[_]>>();

        sources_changed_waker_remote_streams
            .iter_mut()
            .map(
                |(sources_changed_waker_remote, sources_changed_waker_remote_stream)| {
                    let sources_changed_waker_remote = *sources_changed_waker_remote;

                    sources_changed_waker_remote_stream
                        .map(move |()| sources_changed_waker_remote)
                        .boxed() // FIXME: boxed is required because of some problems with unpin
                },
            )
            .collect::<StreamSelectAllOrPending<_>>()
            .stream_take_until_exhausted(exit_flag)
            .ready_chunks_dynamic()
            .map(|sources_changed_waker_remotes| {
                sources_changed_waker_remotes
                    .into_iter()
                    .collect::<HashSet<_>>()
            })
            .for_each(async |sources_changed_waker_remotes| {
                let mut targets_changed_waker_remotes =
                    HashSet::<ByAddress<&TargetsChangedWakerRemote<'d>>>::new();

                for sources_changed_waker_remote in sources_changed_waker_remotes {
                    let (connections_state, connections_event) = self
                        .inner
                        .borrow_child()
                        .connections
                        .get(sources_changed_waker_remote)
                        .unwrap();

                    // state signals
                    for (state_source_remote_base, connection_targets) in connections_state.iter() {
                        let values = state_source_remote_base.take_pending();
                        if values.is_empty() {
                            continue;
                        }

                        for (state_target_remote_base, targets_changed_waker_remote) in
                            connection_targets.iter()
                        {
                            if state_target_remote_base.set(&values) {
                                targets_changed_waker_remotes.insert(*targets_changed_waker_remote);
                            }
                        }
                    }

                    // event connections
                    for (event_source_remote_base, connection_targets) in connections_event.iter() {
                        let values = event_source_remote_base.take_pending();
                        if values.is_empty() {
                            continue;
                        }

                        for (event_target_remote_base, targets_changed_waker_remote) in
                            connection_targets.iter()
                        {
                            if event_target_remote_base.push(&values) {
                                targets_changed_waker_remotes.insert(*targets_changed_waker_remote);
                            }
                        }
                    }
                }

                for targets_changed_waker_remote in targets_changed_waker_remotes {
                    targets_changed_waker_remote.wake();
                }
            })
            .await;

        Exited
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.sources_to_targets_all_run().await;
        let _: Exited = self.sources_to_targets_wakers_run(exit_flag).await;

        Exited
    }
}
#[async_trait]
impl Runnable for Exchanger<'_> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

fn new_inner<'d>(
    devices: &HashMap<DeviceId, DeviceBaseRef<'d>>,
    connections_requested: &[ConnectionRequested],
) -> Result<ExchangerInner<'d>, Error> {
    let inner = ExchangerInner::try_new(
        new_inner_parent(devices).context("new_inner_parent")?,
        |parent| -> Result<_, Error> {
            let child =
                new_inner_child(parent, connections_requested).context("new_inner_child")?;
            Ok(child)
        },
    )
    .context("try_new")?;

    Ok(inner)
}
fn new_inner_parent<'d>(
    devices: &HashMap<DeviceId, DeviceBaseRef<'d>>
) -> Result<ExchangerInnerParent<'d>, Error> {
    let mut signals = HashSet::<ByAddress<&'d dyn Base>>::new();

    // filled from device signals
    // for devices with at least one target TargetsChangedRemote must be not None
    // for devices with at least one source SourcesChangedRemote must be not None
    // this is checked later
    let device_contexts: HashMap<
        DeviceId,
        (
            DeviceBaseRef<'d>,
            Option<TargetsChangedWakerRemote<'d>>,
            Option<SourcesChangedWakerRemote<'d>>,
            HashMap<IdentifierBaseWrapper, &'d dyn RemoteBase>,
        ),
    > = devices
        .iter()
        .map(|(device_id, device)| {
            let targets_changed_waker_remote = device
                .targets_changed_waker()
                .map(|targets_changed_waker| targets_changed_waker.remote());

            let sources_changed_waker_remote = device
                .sources_changed_waker()
                .map(|sources_changed_waker| sources_changed_waker.remote());

            let signals_by_identifier = device.by_identifier();
            for (signal_identifier, signal) in signals_by_identifier.iter() {
                if !signals.insert(ByAddress(*signal)) {
                    panic!(
                        "signal {:?} of device #{} ({}) is returned twice",
                        signal_identifier,
                        device_id,
                        device.type_name()
                    );
                }
            }
            let signals_remote_base_by_identifier = signals_by_identifier
                .into_iter()
                .map(|(signal_identifier, signal)| (signal_identifier, signal.as_remote_base()))
                .collect::<HashMap<_, _>>();

            (
                *device_id,
                (
                    *device,
                    targets_changed_waker_remote,
                    sources_changed_waker_remote,
                    signals_remote_base_by_identifier,
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    Ok(ExchangerInnerParent { device_contexts })
}
fn new_inner_child<'p, 'd>(
    parent: &'p ExchangerInnerParent<'d>,
    connections_requested: &[ConnectionRequested],
) -> Result<ExchangerInnerChild<'p, 'd>, Error> {
    // list of disconnected targets (with no source)
    // used to be set to None during initialization
    let mut state_targets_disconnected = HashMap::<
        ByAddress<&dyn StateTargetRemoteBase>,
        ByAddress<&TargetsChangedWakerRemote<'d>>,
    >::new();

    // list of all connections
    // level 1 and 2 are preinitialized (this is also a constraint for a class)
    let mut connections = HashMap::<
        ByAddress<&SourcesChangedWakerRemote<'d>>,
        (
            HashMap<
                ByAddress<&dyn StateSourceRemoteBase>,
                HashMap<
                    ByAddress<&dyn StateTargetRemoteBase>,
                    ByAddress<&TargetsChangedWakerRemote<'d>>,
                >,
            >,
            HashMap<
                ByAddress<&dyn EventSourceRemoteBase>,
                HashMap<
                    ByAddress<&dyn EventTargetRemoteBase>,
                    ByAddress<&TargetsChangedWakerRemote<'d>>,
                >,
            >,
        ),
    >::new();

    // check if each device with targets has targets waker and each device with
    // source has source waker fill state_targets_disconnected with all signals,
    // will be removed later fill connections with sources for all signals
    for (
        device_id,
        (device, targets_changed_waker_remote, sources_changed_waker_remote, signals_by_identifier),
    ) in parent.device_contexts.iter()
    {
        for remote_base in signals_by_identifier.values() {
            let remote_base_variant = remote_base.as_remote_base_variant();

            // check for waker in direction
            match remote_base_variant {
                RemoteBaseVariant::StateSource(_) | RemoteBaseVariant::EventSource(_) => {
                    let sources_changed_waker_remote = match sources_changed_waker_remote {
                        Some(sources_changed_waker_remote) => sources_changed_waker_remote,
                        None => panic!(
                            "missing source waker for device #{} ({}) with sources",
                            device_id,
                            device.type_name(),
                        ),
                    };

                    let (connections_state, connections_event) = connections
                        .entry(ByAddress(sources_changed_waker_remote))
                        .or_default();

                    match remote_base_variant {
                        RemoteBaseVariant::StateSource(state_source_remote_base) => {
                            connections_state
                                .entry(ByAddress(state_source_remote_base))
                                .or_default();
                        }
                        RemoteBaseVariant::EventSource(event_source_remote_base) => {
                            connections_event
                                .entry(ByAddress(event_source_remote_base))
                                .or_default();
                        }
                        // not possible
                        RemoteBaseVariant::StateTarget(_) | RemoteBaseVariant::EventTarget(_) => {
                            panic!()
                        }
                    }
                }
                RemoteBaseVariant::StateTarget(_) | RemoteBaseVariant::EventTarget(_) => {
                    let _targets_changed_waker_remote = match targets_changed_waker_remote {
                        Some(targets_changed_waker_remote) => targets_changed_waker_remote,
                        None => panic!(
                            "missing target waker for device #{} ({}) with targets",
                            device_id,
                            device.type_name(),
                        ),
                    };
                }
            }

            // prepare list of unused targets
            if let RemoteBaseVariant::StateTarget(state_target_remote_base) = remote_base_variant {
                state_targets_disconnected.insert(
                    ByAddress(state_target_remote_base),
                    ByAddress(targets_changed_waker_remote.as_ref().unwrap()), // this is checked above
                );
            }
        }
    }

    // list of connected targets
    // used to ensure that each target has at most one source
    let mut state_targets_connected = HashSet::<ByAddress<&dyn StateTargetRemoteBase>>::new();

    // list of used connections
    // used to make sure there are no stupid duplicates
    // for state targets this is done via state_targets_connected
    let mut event_connections = HashSet::<(
        ByAddress<&dyn EventSourceRemoteBase>,
        ByAddress<&dyn EventTargetRemoteBase>,
    )>::new();

    // connections processing loop
    for (source_device_id_signal_identifier_base, target_device_id_signal_identifier_base) in
        connections_requested
    {
        // source device and signal
        let (source_device, _, source_sources_changed_waker_remote, source_signals_by_identifier) =
            parent
                .device_contexts
                .get(&source_device_id_signal_identifier_base.device_id)
                .ok_or_else(|| {
                    anyhow!(
                        "source device #{} not found",
                        source_device_id_signal_identifier_base.device_id
                    )
                })?;

        let source_signal_remote_base = source_signals_by_identifier
            .get(&source_device_id_signal_identifier_base.signal_identifier_base_wrapper)
            .ok_or_else(|| {
                anyhow!(
                    "signal {:?} not found on source device #{} ({})",
                    &source_device_id_signal_identifier_base.signal_identifier_base_wrapper,
                    &source_device_id_signal_identifier_base.device_id,
                    source_device.type_name(),
                )
            })?;

        // target device and signal
        let (
            target_device,
            target_targets_changed_waker_remote,
            _,
            target_remote_bases_by_identifier,
        ) = parent
            .device_contexts
            .get(&target_device_id_signal_identifier_base.device_id)
            .ok_or_else(|| {
                anyhow!(
                    "target device {} not found",
                    &target_device_id_signal_identifier_base.device_id
                )
            })?;

        let target_remote_base_remote_base = target_remote_bases_by_identifier
            .get(&target_device_id_signal_identifier_base.signal_identifier_base_wrapper)
            .ok_or_else(|| {
                anyhow!(
                    "signal {:?} not found on target device #{} ({})",
                    &target_device_id_signal_identifier_base.signal_identifier_base_wrapper,
                    &target_device_id_signal_identifier_base.device_id,
                    target_device.type_name()
                )
            })?;

        // connection
        ensure!(
            source_signal_remote_base.type_id() == target_remote_base_remote_base.type_id(),
            "source #{} ({}) :: {:?} -> target #{} ({}) :: {:?} type mismatch: {} -> {}",
            &source_device_id_signal_identifier_base.device_id,
            source_device.type_name(),
            &source_device_id_signal_identifier_base.signal_identifier_base_wrapper,
            &target_device_id_signal_identifier_base.device_id,
            target_device.type_name(),
            &target_device_id_signal_identifier_base.signal_identifier_base_wrapper,
            source_signal_remote_base.type_name(),
            target_remote_base_remote_base.type_name(),
        );

        match (
            source_signal_remote_base.as_remote_base_variant(),
            target_remote_base_remote_base.as_remote_base_variant(),
        ) {
            (
                RemoteBaseVariant::StateSource(state_source_remote_base),
                RemoteBaseVariant::StateTarget(state_target_remote_base),
            ) => {
                // this is checked during signals iteration
                let source_sources_changed_waker_remote =
                    source_sources_changed_waker_remote.as_ref().unwrap();
                let target_targets_changed_waker_remote =
                    target_targets_changed_waker_remote.as_ref().unwrap();

                // make sure the target does not have multiple sources
                ensure!(
                    state_targets_connected.insert(ByAddress(state_target_remote_base)),
                    "multiple sources for target #{} ({}) :: {:?}",
                    &target_device_id_signal_identifier_base.device_id,
                    target_device.type_name(),
                    &target_device_id_signal_identifier_base.signal_identifier_base_wrapper,
                );

                // remove this signal from disconnected list, as its connected nows
                state_targets_disconnected.remove(&ByAddress(state_target_remote_base));

                // add connection
                let (connections_state, _) = connections
                    .get_mut(&ByAddress(source_sources_changed_waker_remote))
                    .unwrap(); // this is guaranteed during device iteration

                connections_state
                    .get_mut(&ByAddress(state_source_remote_base))
                    .unwrap() // this is guaranteed during device iteration
                    .insert(
                        ByAddress(state_target_remote_base),
                        ByAddress(target_targets_changed_waker_remote),
                    );
            }
            (
                RemoteBaseVariant::EventSource(event_source_remote_base),
                RemoteBaseVariant::EventTarget(event_target_remote_base),
            ) => {
                // this is checked during signals iteration
                let source_sources_changed_waker_remote =
                    source_sources_changed_waker_remote.as_ref().unwrap();
                let target_targets_changed_waker_remote =
                    target_targets_changed_waker_remote.as_ref().unwrap();

                // make sure the signal is not duplicated
                ensure!(
                    event_connections.insert((
                        ByAddress(event_source_remote_base),
                        ByAddress(event_target_remote_base),
                    )),
                    "duplicated connection #{} ({}) :: {:?} -> #{} ({}) :: {:?}",
                    &source_device_id_signal_identifier_base.device_id,
                    source_device.type_name(),
                    &source_device_id_signal_identifier_base.signal_identifier_base_wrapper,
                    &target_device_id_signal_identifier_base.device_id,
                    target_device.type_name(),
                    &target_device_id_signal_identifier_base.signal_identifier_base_wrapper,
                );

                // add connection
                let (_, connections_event) = connections
                    .get_mut(&ByAddress(source_sources_changed_waker_remote))
                    .unwrap(); // this is guaranteed during device iteration

                connections_event
                    .get_mut(&ByAddress(event_source_remote_base))
                    .unwrap() // this is guaranteed during device iteration
                    .insert(
                        ByAddress(event_target_remote_base),
                        ByAddress(target_targets_changed_waker_remote),
                    );
            }
            (RemoteBaseVariant::StateTarget(_) | RemoteBaseVariant::EventTarget(_), _)
            | (_, RemoteBaseVariant::StateSource(_) | RemoteBaseVariant::EventSource(_)) => {
                bail!(
                    "signal direction mismatch #{} ({}) :: {:?} -> #{} ({}) :: {:?}",
                    &source_device_id_signal_identifier_base.device_id,
                    source_device.type_name(),
                    &source_device_id_signal_identifier_base.signal_identifier_base_wrapper,
                    &target_device_id_signal_identifier_base.device_id,
                    target_device.type_name(),
                    &target_device_id_signal_identifier_base.signal_identifier_base_wrapper,
                );
            }
            (RemoteBaseVariant::StateSource(_), RemoteBaseVariant::EventTarget(_))
            | (RemoteBaseVariant::EventSource(_), RemoteBaseVariant::StateTarget(_)) => {
                bail!(
                    "signal class mismatch #{} ({}) :: {:?} -> #{} ({}) :: {:?}",
                    &source_device_id_signal_identifier_base.device_id,
                    source_device.type_name(),
                    &source_device_id_signal_identifier_base.signal_identifier_base_wrapper,
                    &target_device_id_signal_identifier_base.device_id,
                    target_device.type_name(),
                    &target_device_id_signal_identifier_base.signal_identifier_base_wrapper,
                );
            }
        }
    }

    Ok(ExchangerInnerChild {
        connections,
        state_targets_disconnected,
    })
}
