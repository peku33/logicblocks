use super::{
    channel::Channel,
    manager::{ChannelId, ChannelIdSegment, Manager},
};
use crate::{
    modules::{fs::Fs, module_path::ModulePath},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        drop_guard::DropGuard,
        runnable::{Exited, Runnable},
        runtime::{Runtime, RuntimeScope, RuntimeScopeRunnable},
    },
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use futures::{
    channel::mpsc::UnboundedSender,
    future::{FutureExt, JoinAll},
    join,
    stream::StreamExt,
};
use once_cell::sync::Lazy;
use ouroboros::self_referencing;
use std::{
    collections::HashMap,
    mem::{transmute, ManuallyDrop},
    time::Duration,
};
use tokio::sync::{RwLock, RwLockReadGuard};

type RunnerChannelRunners<'r> = HashMap<ChannelId, RunnerChannelRunner<'r>>;

#[derive(Debug)]
pub struct RunnerChannelsLock<'a, 'r> {
    inner: RwLockReadGuard<'a, RunnerChannelRunners<'r>>,
}
impl<'a, 'r> RunnerChannelsLock<'a, 'r> {
    fn new(inner: RwLockReadGuard<'a, RunnerChannelRunners<'r>>) -> Self {
        Self { inner }
    }

    pub fn channels(&self) -> HashMap<ChannelId, &RunnerChannel> {
        self.inner
            .iter()
            .map(|(channel_id, runner_channel_runner)| {
                (*channel_id, runner_channel_runner.runner_channel())
            })
            .collect::<HashMap<ChannelId, &RunnerChannel>>()
    }
}

#[derive(Debug)]
pub struct RunnerChannel {
    channel_id: ChannelId,
    channel_name: String,
    channel: Channel,
    manager_channel_segment_sender: UnboundedSender<ChannelIdSegment>,
}
impl RunnerChannel {
    fn new(
        channel_id: ChannelId,
        channel_name: String,
        channel: Channel,
        manager: &Manager<'_>,
    ) -> Self {
        let manager_channel_segment_sender = manager.channel_segment_sender();

        Self {
            channel_id,
            channel_name,
            channel,
            manager_channel_segment_sender,
        }
    }

    pub fn channel_id(&self) -> &ChannelId {
        &self.channel_id
    }
    pub fn channel_name(&self) -> &str {
        &self.channel_name
    }
    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    async fn channel_segment_forwarder_run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.channel
            .channel_segment_receiver_borrow_mut()
            .by_ref()
            .stream_take_until_exhausted(exit_flag)
            .for_each(async |channel_segment| {
                let channel_id_segment = ChannelIdSegment {
                    id: self.channel_id,
                    segment: channel_segment,
                };
                self.manager_channel_segment_sender
                    .unbounded_send(channel_id_segment)
                    .unwrap();
            })
            .await;

        Exited
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        let channel_run_exit_flag_receiver = exit_flag;
        let (
            channel_segment_forwarder_exit_flag_sender,
            channel_segment_forwarder_exit_flag_receiver,
        ) = async_flag::pair();

        let channel_runner =
            self.channel
                .run(channel_run_exit_flag_receiver)
                .then(async |Exited| {
                    channel_segment_forwarder_exit_flag_sender.signal();
                });
        let channel_segment_forwarder_runner =
            self.channel_segment_forwarder_run(channel_segment_forwarder_exit_flag_receiver);

        let _: ((), Exited) = join!(channel_runner, channel_segment_forwarder_runner);

        Exited
    }
}
#[async_trait]
impl Runnable for RunnerChannel {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

#[self_referencing]
#[derive(Debug)]
struct RunnerChannelRunnerInner<'r> {
    runner_channel: RunnerChannel,

    #[borrows(runner_channel)]
    #[not_covariant]
    runner_channel_runtime_scope_runnable:
        ManuallyDrop<RuntimeScopeRunnable<'r, 'this, RunnerChannel>>,
}
#[derive(Debug)]
struct RunnerChannelRunner<'r> {
    inner: RunnerChannelRunnerInner<'r>,

    drop_guard: DropGuard,
}
impl<'r> RunnerChannelRunner<'r> {
    fn new(
        runtime: &'r Runtime,
        runner_channel: RunnerChannel,
    ) -> Self {
        let inner = RunnerChannelRunnerInnerBuilder {
            runner_channel,
            runner_channel_runtime_scope_runnable_builder: |runner_channel| {
                let runner_channel_runtime_scope_runnable =
                    RuntimeScopeRunnable::new(runtime, runner_channel);
                let runner_channel_runtime_scope_runnable =
                    ManuallyDrop::new(runner_channel_runtime_scope_runnable);
                runner_channel_runtime_scope_runnable
            },
        }
        .build();

        let drop_guard = DropGuard::new();

        Self { inner, drop_guard }
    }

    pub fn runner_channel(&self) -> &RunnerChannel {
        self.inner.borrow_runner_channel()
    }

    async fn finalize(mut self) -> RunnerChannel {
        let runner_channel_runtime_scope_runnable =
            self.inner.with_runner_channel_runtime_scope_runnable_mut(
                |runner_channel_runtime_scope_runnable| {
                    let runner_channel_runtime_scope_runnable = unsafe {
                        transmute::<
                            &mut ManuallyDrop<RuntimeScopeRunnable<'_, '_, RunnerChannel>>,
                            &mut ManuallyDrop<
                                RuntimeScopeRunnable<'static, 'static, RunnerChannel>,
                            >,
                        >(runner_channel_runtime_scope_runnable)
                    };
                    let runner_channel_runtime_scope_runnable =
                        unsafe { ManuallyDrop::take(runner_channel_runtime_scope_runnable) };
                    runner_channel_runtime_scope_runnable
                },
            );
        runner_channel_runtime_scope_runnable.finalize().await;

        self.drop_guard.set();

        let inner_heads = self.inner.into_heads();
        inner_heads.runner_channel
    }
}

#[self_referencing]
#[derive(Debug)]
struct ManagerRunnerInner<'r, 'f: 'r> {
    manager: Manager<'f>,

    #[borrows(manager)]
    #[not_covariant]
    manager_runtime_scope_runnable: ManuallyDrop<RuntimeScopeRunnable<'r, 'this, Manager<'f>>>,
}
#[derive(Debug)]
struct ManagerRunner<'r, 'f> {
    inner: ManagerRunnerInner<'r, 'f>,

    drop_guard: DropGuard,
}
impl<'r, 'f> ManagerRunner<'r, 'f> {
    fn new(
        runtime: &'r Runtime,
        manager: Manager<'f>,
    ) -> Self {
        let inner = ManagerRunnerInnerBuilder {
            manager,
            manager_runtime_scope_runnable_builder: |manager| {
                let manager_runtime_scope_runnable = RuntimeScopeRunnable::new(runtime, manager);
                let manager_runtime_scope_runnable =
                    ManuallyDrop::new(manager_runtime_scope_runnable);
                manager_runtime_scope_runnable
            },
        }
        .build();

        let drop_guard = DropGuard::new();

        Self { inner, drop_guard }
    }

    pub fn manager(&self) -> &Manager<'f> {
        self.inner.borrow_manager()
    }

    async fn finalize(mut self) -> Manager<'f> {
        let manager_runtime_scope_runnable =
            self.inner
                .with_manager_runtime_scope_runnable_mut(|manager_runtime_scope_runnable| {
                    let manager_runtime_scope_runnable = unsafe {
                        transmute::<
                            &mut ManuallyDrop<RuntimeScopeRunnable<'_, '_, Manager<'_>>>,
                            &mut ManuallyDrop<
                                RuntimeScopeRunnable<'static, 'static, Manager<'static>>,
                            >,
                        >(manager_runtime_scope_runnable)
                    };
                    let manager_runtime_scope_runnable =
                        unsafe { ManuallyDrop::take(manager_runtime_scope_runnable) };
                    manager_runtime_scope_runnable
                });
        manager_runtime_scope_runnable.finalize().await;

        self.drop_guard.set();

        let inner = self.inner.into_heads();
        inner.manager
    }
}

#[derive(Debug)]
pub struct Runner<'r, 'f> {
    runtime: &'r Runtime,

    manager_runner: ManagerRunner<'r, 'f>,
    runner_channel_runners: RwLock<RunnerChannelRunners<'r>>,
}
impl<'r, 'f> Runner<'r, 'f> {
    pub const SEGMENT_TIME: Duration = Duration::from_secs(60);

    pub fn new(
        runtime: &'r Runtime,
        name: String,
        fs: &'f Fs,
    ) -> Self {
        let manager = Manager::new(name, fs);
        let manager_runner = ManagerRunner::new(runtime, manager);

        let runner_channel_runners = RunnerChannelRunners::default();
        let runner_channel_runners = RwLock::new(runner_channel_runners);

        Self {
            runtime,

            manager_runner,
            runner_channel_runners,
        }
    }

    pub async fn channels_reload(&self) -> Result<(), Error> {
        // lock channels
        let mut runner_channel_runners = self
            .runner_channel_runners
            .try_write()
            .context("runner_channel_runners lock")?;

        // finalize current channels
        runner_channel_runners
            .drain()
            .map(|(_, runner_channel)| runner_channel.finalize())
            .collect::<JoinAll<_>>()
            .await;

        // get new channels data
        let channels_data = self
            .manager_runner
            .manager()
            .channels_data_get()
            .await
            .context("manager channels_data_get")?;

        // create & run new channels
        *runner_channel_runners = channels_data
            .into_iter()
            .map(|(channel_id, channel_data)| {
                let temporary_storage_directory = self
                    .manager_runner
                    .manager()
                    .channel_temporary_directory_path_build(channel_id);

                let channel = Channel::new(
                    None,
                    Self::SEGMENT_TIME,
                    temporary_storage_directory,
                    channel_data.detection_threshold,
                );
                let runner_channel = RunnerChannel::new(
                    channel_id,
                    channel_data.name,
                    channel,
                    self.manager_runner.manager(),
                );
                let runner_channel_runner = RunnerChannelRunner::new(self.runtime, runner_channel);
                (channel_id, runner_channel_runner)
            })
            .collect();

        // drop lock
        drop(runner_channel_runners);

        Ok(())
    }
    pub fn channels_lock(&self) -> Option<RunnerChannelsLock<'_, 'r>> {
        let runner_channel_runners_lock = self.runner_channel_runners.try_read().ok()?;
        let runner_channels_lock = RunnerChannelsLock::new(runner_channel_runners_lock);
        Some(runner_channels_lock)
    }

    pub async fn finalize(self) {
        self.runner_channel_runners
            .into_inner()
            .into_values()
            .map(|runner_channel| runner_channel.finalize())
            .collect::<JoinAll<_>>()
            .await;

        // manager will still collect recordings created by closing channels
        self.manager_runner.finalize().await;
    }
}

#[self_referencing]
#[derive(Debug)]
struct RunnerOwnedInner<'f> {
    runtime: Runtime,

    #[borrows(runtime)]
    #[not_covariant]
    runner: ManuallyDrop<Runner<'this, 'f>>,

    #[borrows(runtime, runner)]
    #[not_covariant]
    runner_runtime_scope: ManuallyDrop<RuntimeScope<'this, 'this, Runner<'this, 'f>>>,
}
#[derive(Debug)]
pub struct RunnerOwned<'f> {
    inner: RunnerOwnedInner<'f>,

    drop_guard: DropGuard,
}
impl<'f> RunnerOwned<'f> {
    fn module_path() -> &'static ModulePath {
        static MODULE_PATH: Lazy<ModulePath> =
            Lazy::new(|| ModulePath::new(&["devices", "soft", "surveillance", "rtsp_recorder"]));
        &MODULE_PATH
    }

    pub fn new(
        name: String,
        fs: &'f Fs,
    ) -> Self {
        let runtime = Runtime::new(Self::module_path(), 1, 1);

        let inner = RunnerOwnedInnerBuilder {
            runtime,

            runner_builder: |runtime| {
                let runner = Runner::new(runtime, name, fs);
                let runner = ManuallyDrop::new(runner);
                runner
            },

            runner_runtime_scope_builder: |runtime, runner| {
                let runner_runtime_scope = RuntimeScope::new(runtime, &**runner);
                let runner_runtime_scope = ManuallyDrop::new(runner_runtime_scope);
                runner_runtime_scope
            },
        }
        .build();

        let drop_guard = DropGuard::new();

        Self { inner, drop_guard }
    }

    pub async fn channels_reload(&self) -> Result<(), Error> {
        let runner_runtime_scope: &RuntimeScope<'f, 'f, Runner<'f, 'f>> = self
            .inner
            .with_runner_runtime_scope(|runner_runtime_scope| unsafe {
                #[allow(clippy::transmute_ptr_to_ptr)]
                transmute::<
                    &RuntimeScope<'_, '_, Runner<'_, '_>>,
                    &RuntimeScope<'f, 'f, Runner<'f, 'f>>,
                >(runner_runtime_scope)
            });
        runner_runtime_scope
            .execute(|runner| runner.channels_reload().boxed())
            .await
    }
    pub fn channels_lock(&self) -> Option<RunnerChannelsLock<'_, '_>> {
        let runner: &Runner<'_, '_> = self.inner.with_runner(|runner| unsafe {
            #[allow(clippy::transmute_ptr_to_ptr)]
            transmute::<&Runner<'_, '_>, &Runner<'static, 'static>>(runner)
        });
        runner.channels_lock()
    }

    pub async fn finalize(self) {
        let runner_runtime_scope = self
            .inner
            .with_runner_runtime_scope(|runner_runtime_scope| {
                #[allow(mutable_transmutes)]
                let runner_runtime_scope = unsafe {
                    #[allow(clippy::transmute_ptr_to_ptr)]
                    transmute::<
                        &ManuallyDrop<RuntimeScope<'_, '_, Runner<'_, '_>>>,
                        &mut ManuallyDrop<RuntimeScope<'f, 'f, Runner<'f, 'f>>>,
                    >(runner_runtime_scope)
                };
                let runner_runtime_scope = unsafe { ManuallyDrop::take(runner_runtime_scope) };
                runner_runtime_scope
            });
        runner_runtime_scope.finalize().await;

        let runner = self.inner.with_runner(|runner| {
            #[allow(mutable_transmutes)]
            let runner = unsafe {
                #[allow(clippy::transmute_ptr_to_ptr)]
                transmute::<&ManuallyDrop<Runner<'_, '_>>, &mut ManuallyDrop<Runner<'f, 'f>>>(
                    runner,
                )
            };
            let runner = unsafe { ManuallyDrop::take(runner) };
            runner
        });
        runner.finalize().await;

        self.drop_guard.set();

        let inner_heads = self.inner.into_heads();
        drop(inner_heads);
    }
}
