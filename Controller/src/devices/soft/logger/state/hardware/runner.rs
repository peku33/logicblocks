use super::{
    manager::{Manager, SinkData, SinkId, SinkItem},
    sink::SinkBase,
};
use crate::{
    modules::fs::Fs,
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runtime::{Exited, FinalizeGuard, Runnable, Runtime, RuntimeScope, RuntimeScopeRunnable},
    },
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use crossbeam::channel;
use futures::{
    future::{FutureExt, JoinAll},
    join,
    stream::StreamExt,
};
use ouroboros::self_referencing;
use std::{
    collections::HashMap,
    mem,
    mem::{transmute, ManuallyDrop},
};
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Debug)]
pub struct RunnerSink {
    sink_id: SinkId,
    sink_name: String,
    sink_base: SinkBase,
    manager_sink_items_sender: channel::Sender<SinkItem>,
}
impl RunnerSink {
    fn new(
        sink_id: SinkId,
        sink_name: String,
        sink_base: SinkBase,
        manager: &Manager,
    ) -> Self {
        let manager_sink_items_sender = manager.sink_items_sender_get();

        Self {
            sink_id,
            sink_name,
            sink_base,
            manager_sink_items_sender,
        }
    }

    pub fn sink_id(&self) -> &SinkId {
        &self.sink_id
    }
    pub fn sink_name(&self) -> &str {
        &self.sink_name
    }
    pub fn sink_base(&self) -> &SinkBase {
        &self.sink_base
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        let mut sink_items_receiver = self.sink_base.items_receiver_lease();

        let sink_items_receiver_runner = sink_items_receiver
            .by_ref()
            .stream_take_until_exhausted(exit_flag)
            .for_each(async move |time_value| {
                let sink_item = SinkItem {
                    sink_id: self.sink_id,
                    time_value,
                };
                self.manager_sink_items_sender.send(sink_item).unwrap();
            })
            .boxed();

        let _: ((),) = join!(sink_items_receiver_runner);

        Exited
    }
}
#[async_trait]
impl Runnable for RunnerSink {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

#[self_referencing]
#[derive(Debug)]
struct RunnerSinkRunnerInner<'r> {
    runner_sink: RunnerSink,

    #[borrows(runner_sink)]
    #[not_covariant]
    runner_sink_runtime_scope_runnable: ManuallyDrop<RuntimeScopeRunnable<'r, 'this, RunnerSink>>,
}
#[derive(Debug)]
struct RunnerSinkRunner<'r> {
    inner: RunnerSinkRunnerInner<'r>,

    finalize_guard: FinalizeGuard,
}
impl<'r> RunnerSinkRunner<'r> {
    fn new(
        runner_sink: RunnerSink,
        runtime: &'r Runtime,
    ) -> Self {
        let inner = RunnerSinkRunnerInner::new(runner_sink, move |runner_sink| {
            let runner_sink_runtime_scope_runnable =
                RuntimeScopeRunnable::new(runtime, runner_sink);
            let runner_sink_runtime_scope_runnable =
                ManuallyDrop::new(runner_sink_runtime_scope_runnable);
            runner_sink_runtime_scope_runnable
        });

        let finalize_guard = FinalizeGuard::new();

        Self {
            inner,
            finalize_guard,
        }
    }

    pub fn runner_sink(&self) -> &RunnerSink {
        self.inner.borrow_runner_sink()
    }

    async fn finalize(mut self) -> RunnerSink {
        let runner_sink_runtime_scope_runnable =
            self.inner.with_runner_sink_runtime_scope_runnable_mut(
                move |runner_sink_runtime_scope_runnable| {
                    let runner_sink_runtime_scope_runnable = unsafe {
                        transmute::<
                            &mut ManuallyDrop<RuntimeScopeRunnable<'_, '_, RunnerSink>>,
                            &mut ManuallyDrop<RuntimeScopeRunnable<'static, 'static, RunnerSink>>,
                        >(runner_sink_runtime_scope_runnable)
                    };
                    let runner_sink_runtime_scope_runnable =
                        unsafe { ManuallyDrop::take(runner_sink_runtime_scope_runnable) };
                    runner_sink_runtime_scope_runnable
                },
            );
        runner_sink_runtime_scope_runnable.finalize().await;

        self.finalize_guard.finalized();

        let inner_heads = self.inner.into_heads();
        inner_heads.runner_sink
    }
}

#[derive(Debug)]
struct RunnerSinksRunner<'r> {
    inner: HashMap<SinkId, RunnerSinkRunner<'r>>,
}
impl<'r> RunnerSinksRunner<'r> {
    fn new(inner: HashMap<SinkId, RunnerSinkRunner<'r>>) -> Self {
        Self { inner }
    }
    fn empty() -> Self {
        Self::new(HashMap::<SinkId, RunnerSinkRunner<'r>>::new())
    }

    pub fn runner_sinks(&self) -> HashMap<SinkId, &RunnerSink> {
        self.inner
            .iter()
            .map(|(sink_id, runner_sink)| (*sink_id, runner_sink.runner_sink()))
            .collect::<HashMap<_, _>>()
    }

    async fn finalize(self) {
        self.inner
            .into_iter()
            .map(move |(_sink_id, runner_sink_runner)| runner_sink_runner.finalize())
            .collect::<JoinAll<_>>()
            .await;
    }
}

#[derive(Debug)]
pub struct RunnerSinksLock<'a, 'r> {
    inner: RwLockReadGuard<'a, RunnerSinksRunner<'r>>,
}
impl<'a, 'r> RunnerSinksLock<'a, 'r> {
    fn new(inner: RwLockReadGuard<'a, RunnerSinksRunner<'r>>) -> Self {
        Self { inner }
    }
    pub fn runner_sinks(&self) -> HashMap<SinkId, &RunnerSink> {
        self.inner.runner_sinks()
    }
}

#[self_referencing]
#[derive(Debug)]
struct ManagerRunnerInner<'f: 'r, 'r> {
    manager: Manager<'f>,

    #[borrows(manager)]
    #[not_covariant]
    manager_runtime_scope_runnable: ManuallyDrop<RuntimeScopeRunnable<'r, 'this, Manager<'f>>>,
}
#[derive(Debug)]
struct ManagerRunner<'f: 'r, 'r> {
    inner: ManagerRunnerInner<'f, 'r>,

    finalize_guard: FinalizeGuard,
}
impl<'f: 'r, 'r> ManagerRunner<'f, 'r> {
    fn new(
        manager: Manager<'f>,
        runtime: &'r Runtime,
    ) -> Self {
        let inner = ManagerRunnerInner::new(manager, move |manager| {
            let manager_runtime_scope_runnable = RuntimeScopeRunnable::new(runtime, manager);
            let manager_runtime_scope_runnable = ManuallyDrop::new(manager_runtime_scope_runnable);
            manager_runtime_scope_runnable
        });

        let finalize_guard = FinalizeGuard::new();

        Self {
            inner,
            finalize_guard,
        }
    }

    pub fn manager(&self) -> &Manager<'f> {
        self.inner.borrow_manager()
    }

    async fn finalize(mut self) -> Manager<'f> {
        let manager_runtime_scope_runnable = self.inner.with_manager_runtime_scope_runnable_mut(
            move |manager_runtime_scope_runnable| {
                let manager_runtime_scope_runnable = unsafe {
                    transmute::<
                        &mut ManuallyDrop<RuntimeScopeRunnable<'_, '_, Manager<'_>>>,
                        &mut ManuallyDrop<RuntimeScopeRunnable<'static, 'static, Manager<'static>>>,
                    >(manager_runtime_scope_runnable)
                };
                let manager_runtime_scope_runnable =
                    unsafe { ManuallyDrop::take(manager_runtime_scope_runnable) };
                manager_runtime_scope_runnable
            },
        );
        manager_runtime_scope_runnable.finalize().await;

        self.finalize_guard.finalized();

        let inner = self.inner.into_heads();
        inner.manager
    }
}

#[derive(Debug)]
pub struct Runner<'f: 'r, 'r> {
    runtime: &'r Runtime,

    manager_runner: ManagerRunner<'f, 'r>,
    runner_sinks_runner: RwLock<RunnerSinksRunner<'r>>,
}
impl<'f: 'r, 'r> Runner<'f, 'r> {
    pub fn new(
        name: String,
        fs: &'f Fs,
        runtime: &'r Runtime,
    ) -> Self {
        let manager = Manager::new(name, fs);
        let manager_runner = ManagerRunner::new(manager, runtime);

        let runner_sinks_runner = RunnerSinksRunner::empty();
        let runner_sinks_runner = RwLock::new(runner_sinks_runner);

        Self {
            runtime,

            manager_runner,
            runner_sinks_runner,
        }
    }

    pub async fn sinks_data_set(
        &self,
        sinks_data: HashMap<SinkId, SinkData>,
    ) -> Result<(), Error> {
        // unload currently running channels
        let mut runner_sinks_runner_lock = self
            .runner_sinks_runner
            .try_write()
            .context("runner_sinks_runner lock")?;

        let runner_sinks_runner =
            mem::replace(&mut *runner_sinks_runner_lock, RunnerSinksRunner::empty());
        runner_sinks_runner.finalize().await;

        self.manager_runner
            .manager()
            .sinks_data_set(sinks_data)
            .await
            .context("sinks_data_set")?;

        drop(runner_sinks_runner_lock);

        Ok(())
    }
    pub async fn sinks_reload(&self) -> Result<(), Error> {
        let mut runner_sinks_runner_lock = self
            .runner_sinks_runner
            .try_write()
            .context("runner_sinks_runner lock")?;

        // create new channels

        // replace old with empty state
        let runner_sinks_runner =
            mem::replace(&mut *runner_sinks_runner_lock, RunnerSinksRunner::empty());
        runner_sinks_runner.finalize().await;

        // get new channels data
        let sinks_data = self
            .manager_runner
            .manager()
            .sinks_data_details_get()
            .await
            .context("sinks_data_get")?;

        let runner_sink_runners = sinks_data
            .into_iter()
            .map(move |(sink_id, sink_data)| -> Result<_, Error> {
                let sink_base = SinkBase::new(sink_data.class);

                let runner_sink = RunnerSink::new(
                    sink_id,
                    sink_data.name,
                    sink_base,
                    self.manager_runner.manager(),
                );

                let runner_sink_runner = RunnerSinkRunner::new(runner_sink, self.runtime);

                Ok((sink_id, runner_sink_runner))
            })
            .collect::<Result<HashMap<_, _>, Error>>().context("collect")?;

        let runner_sinks_runner = RunnerSinksRunner::new(runner_sink_runners);

        // replace empty state with created channels
        let runner_sinks_runner = mem::replace(&mut *runner_sinks_runner_lock, runner_sinks_runner);
        runner_sinks_runner.finalize().await;

        drop(runner_sinks_runner_lock);

        Ok(())
    }
    pub fn sinks_lock(&self) -> Option<RunnerSinksLock<'f, '_>> {
        let runner_sinks_runner_lock = self.runner_sinks_runner.try_read().ok()?;
        let runner_sinks_lock = RunnerSinksLock::new(runner_sinks_runner_lock);
        Some(runner_sinks_lock)
    }

    pub async fn finalize(self) {
        self.runner_sinks_runner.into_inner().finalize().await;
        self.manager_runner.finalize().await;
    }
}

#[self_referencing]
#[derive(Debug)]
struct RunnerOwnedInner<'f> {
    runtime: Runtime,

    #[borrows(runtime)]
    #[not_covariant]
    runner: ManuallyDrop<Runner<'f, 'this>>,

    #[borrows(runtime, runner)]
    #[not_covariant]
    runner_runtime_scope: ManuallyDrop<RuntimeScope<'this, 'this, Runner<'f, 'this>>>,
}
#[derive(Debug)]
pub struct RunnerOwned<'f> {
    inner: RunnerOwnedInner<'f>,

    finalize_guard: FinalizeGuard,
}
impl<'f> RunnerOwned<'f> {
    pub fn new(
        name: String,
        fs: &'f Fs,
    ) -> Self {
        let runtime = Runtime::new(&format!("{}.state.logger", name), 1, 1);

        let inner = RunnerOwnedInner::new(
            runtime,
            move |runtime| {
                let runner = Runner::new(name, fs, runtime);
                let runner = ManuallyDrop::new(runner);
                runner
            },
            move |runtime, runner| {
                let runner_runtime_scope = RuntimeScope::new(runtime, &**runner);
                let runner_runtime_scope = ManuallyDrop::new(runner_runtime_scope);
                runner_runtime_scope
            },
        );

        let finalize_guard = FinalizeGuard::new();

        Self {
            inner,
            finalize_guard,
        }
    }

    pub async fn sinks_data_set(
        &self,
        sinks_data: HashMap<SinkId, SinkData>,
    ) -> Result<(), Error> {
        let runner_runtime_scope: &RuntimeScope<'f, 'f, Runner<'f, 'f>> = self
            .inner
            .with_runner_runtime_scope(move |runner_runtime_scope| unsafe {
                #[allow(clippy::transmute_ptr_to_ptr)]
                transmute::<
                    &RuntimeScope<'_, '_, Runner<'_, '_>>,
                    &RuntimeScope<'f, 'f, Runner<'f, 'f>>,
                >(runner_runtime_scope)
            });
        runner_runtime_scope
            .execute(move |runner| runner.sinks_data_set(sinks_data).boxed())
            .await
    }
    pub async fn sinks_reload(&self) -> Result<(), Error> {
        let runner_runtime_scope: &RuntimeScope<'f, 'f, Runner<'f, 'f>> = self
            .inner
            .with_runner_runtime_scope(move |runner_runtime_scope| unsafe {
                #[allow(clippy::transmute_ptr_to_ptr)]
                transmute::<
                    &RuntimeScope<'_, '_, Runner<'_, '_>>,
                    &RuntimeScope<'f, 'f, Runner<'f, 'f>>,
                >(runner_runtime_scope)
            });
        runner_runtime_scope
            .execute(move |runner| runner.sinks_reload().boxed())
            .await
    }
    pub fn sinks_lock(&self) -> Option<RunnerSinksLock<'_, '_>> {
        let runner: &Runner<'_, '_> = self.inner.with_runner(move |runner| unsafe {
            #[allow(clippy::transmute_ptr_to_ptr)]
            transmute::<&Runner<'_, '_>, &Runner<'static, 'static>>(runner)
        });
        runner.sinks_lock()
    }

    pub async fn finalize(self) {
        let runner_runtime_scope =
            self.inner
                .with_runner_runtime_scope(move |runner_runtime_scope| {
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

        let runner = self.inner.with_runner(move |runner| {
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

        self.finalize_guard.finalized();

        let inner_heads = self.inner.into_heads();
        drop(inner_heads);
    }
}
