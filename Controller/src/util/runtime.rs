use super::async_flag;
use async_trait::async_trait;
use futures::{
    channel::oneshot,
    future::{BoxFuture, Future, FutureExt, JoinAll},
    join,
};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    mem::{take, transmute},
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio::{
    runtime::{Builder as TokioRuntimeBuilder, Runtime as TokioRuntime},
    task::JoinHandle as TokioJoinHandle,
};

#[derive(Debug)]
pub struct Runtime {
    inner: Option<TokioRuntime>,
}
impl Runtime {
    pub fn new(
        name: &str,
        worker_threads: usize,
        blocking_threads_max: usize,
    ) -> Self {
        let inner = TokioRuntimeBuilder::new_multi_thread()
            .enable_all()
            .thread_name(format!("{}.runtime", name))
            .worker_threads(worker_threads)
            .max_blocking_threads(blocking_threads_max)
            .build()
            .unwrap();
        Self { inner: Some(inner) }
    }
    fn spawn<F>(
        &self,
        future: F,
    ) -> TokioJoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.as_ref().unwrap().spawn(future)
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.inner.take().unwrap().shutdown_background();
    }
}

// =============================================================================

#[derive(Debug)]
struct FinalizeGuardInner {
    finalized: bool,
}
impl FinalizeGuardInner {
    pub fn new() -> Self {
        Self { finalized: false }
    }
    pub fn set_finalized(&mut self) {
        assert_eq!(self.finalized, false, "finalized twice");
        self.finalized = true;
    }
}
impl Drop for FinalizeGuardInner {
    fn drop(&mut self) {
        assert_eq!(self.finalized, true, "never finalized");
    }
}

#[derive(Debug)]
pub struct FinalizeGuard {
    inner: FinalizeGuardInner,
}
impl FinalizeGuard {
    pub fn new() -> Self {
        let inner = FinalizeGuardInner::new();
        Self { inner }
    }
    pub fn finalized(mut self) {
        self.inner.set_finalized();
    }
}

// =============================================================================
#[derive(Debug)]
struct RuntimeScopeContext {
    task_id_next: AtomicUsize,
    tasks: Mutex<HashMap<usize, TokioJoinHandle<()>>>,
}
impl RuntimeScopeContext {
    pub fn new() -> Self {
        let task_id_next = 0;
        let task_id_next = AtomicUsize::new(task_id_next);

        let tasks = HashMap::<usize, TokioJoinHandle<()>>::new();
        let tasks = Mutex::new(tasks);

        Self {
            task_id_next,
            tasks,
        }
    }
    pub fn drain_tasks(&self) -> HashMap<usize, TokioJoinHandle<()>> {
        take(&mut *self.tasks.lock())
    }
}

#[derive(Debug)]
pub struct RuntimeScope<'r, 'o, O> {
    runtime: &'r Runtime,
    owner: &'o O,
    context: Box<RuntimeScopeContext>, // StableDeref
    finalize_guard: FinalizeGuard,
}
impl<'r, 'o, O> RuntimeScope<'r, 'o, O> {
    pub fn new(
        runtime: &'r Runtime,
        owner: &'o O,
    ) -> Self {
        let context = RuntimeScopeContext::new();
        let context = Box::new(context);

        let finalize_guard = FinalizeGuard::new();

        Self {
            runtime,
            owner,
            context,
            finalize_guard,
        }
    }

    pub fn owner(&self) -> &'o O {
        self.owner
    }

    pub fn execute<'s, E, R>(
        &'s self,
        executor: E,
    ) -> impl Future<Output = R> + 's
    where
        E: FnOnce(&'o O) -> BoxFuture<'s, R>,
        R: Send + 'static,
    {
        let future = executor(self.owner);
        // SAFE: self.owner will outlive the future itself
        let future = unsafe { transmute::<BoxFuture<'s, R>, BoxFuture<'static, R>>(future) };

        // SAFE: self.context will outlive the future itself
        let context = unsafe {
            transmute::<&'_ RuntimeScopeContext, &'static RuntimeScopeContext>(&*self.context)
        };

        let task_id = context.task_id_next.fetch_add(1, Ordering::Relaxed);
        let (result_sender, result_receiver) = oneshot::channel::<R>();

        let future = async move {
            // run task to completion
            let result = future.await;

            // propagate the result
            let _ = result_sender.send(result);

            // remove task from the list
            context.tasks.lock().remove(&task_id).unwrap();
        };

        // first lock, to avoid completing future before adding to context
        {
            let mut tasks_lock = context.tasks.lock();
            let task = self.runtime.spawn(future);
            tasks_lock.insert(task_id, task);
        }

        result_receiver.map(|result| result.unwrap())
    }
    pub async fn finalize(self) {
        self.context
            .drain_tasks()
            .into_values()
            .collect::<JoinAll<_>>()
            .await
            .into_iter()
            .for_each(|result| result.unwrap());

        self.finalize_guard.finalized();
    }
}

// =============================================================================
#[derive(Debug)]
pub struct Exited;

#[async_trait]
pub trait Runnable: Send + Sync {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited;
}

#[derive(Debug)]
pub struct RuntimeScopeRunnable<'r, 'o, O>
where
    O: Runnable,
{
    runtime_scope: RuntimeScope<'r, 'o, O>,

    runnable_exit_flag_sender: async_flag::Sender,
    runnable_join_handle: TokioJoinHandle<Exited>,

    finalize_guard: FinalizeGuard,
}
impl<'r, 'o, O> RuntimeScopeRunnable<'r, 'o, O>
where
    O: Runnable,
{
    pub fn new(
        runtime: &'r Runtime,
        runnable: &'o O,
    ) -> Self {
        let runtime_scope = RuntimeScope::new(runtime, runnable);

        let (runnable_exit_flag_sender, runnable_exit_flag_receiver) = async_flag::pair();

        let runnable_runner = runtime_scope.owner().run(runnable_exit_flag_receiver);
        // SAFE: runnable is stable deref + will not outlive self
        let runnable_runner = unsafe {
            transmute::<BoxFuture<'_, Exited>, BoxFuture<'static, Exited>>(runnable_runner)
        };
        let runnable_join_handle = runtime.spawn(runnable_runner);

        let finalize_guard = FinalizeGuard::new();

        Self {
            runtime_scope,
            runnable_exit_flag_sender,
            runnable_join_handle,
            finalize_guard,
        }
    }
    pub async fn finalize(self) {
        self.runnable_exit_flag_sender.signal();
        let ((), result) = join!(self.runtime_scope.finalize(), self.runnable_join_handle);
        result.unwrap();

        self.finalize_guard.finalized();
    }
}
