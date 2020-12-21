use super::{async_flag, Captures};
use async_trait::async_trait;
use futures::{
    channel::oneshot,
    future::{BoxFuture, Future, FutureExt, JoinAll},
};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    mem::{take, transmute},
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio::{
    runtime::{Builder as RuntimeBuilder, Runtime},
    task::JoinHandle,
};

pub type ExitFlag = async_flag::Receiver;
pub struct Exited;

#[async_trait]
pub trait Runnable {
    async fn run(
        &self,
        exit_flag: ExitFlag,
    ) -> Exited;
}

struct TaskDetached {
    exit_flag_sender: async_flag::Sender,
    join_handle: JoinHandle<()>,
}
struct ScopedRuntimeShared {
    token_next: AtomicUsize,
    tasks: Mutex<HashMap<usize, TaskDetached>>,
}
struct ScopedRuntimeInner<O>
where
    O: Sync,
{
    runtime: Runtime,
    shared: ScopedRuntimeShared,
    owner: O,
}
pub struct ScopedRuntime<O>
where
    O: Sync,
{
    inner: Pin<Box<ScopedRuntimeInner<O>>>, // Stable address
}
impl<O> ScopedRuntime<O>
where
    O: Sync,
{
    pub fn new(
        owner: O,
        thread_name: String,
        core_threads: Option<usize>,
        max_threads: Option<usize>,
    ) -> Self {
        let mut runtime_builder = RuntimeBuilder::new();
        runtime_builder
            .enable_all()
            .threaded_scheduler()
            .thread_name(thread_name);
        if let Some(core_threads) = core_threads {
            runtime_builder.core_threads(core_threads);
        }
        if let Some(max_threads) = max_threads {
            runtime_builder.max_threads(max_threads);
        }
        let runtime = runtime_builder.build().unwrap();

        let shared = ScopedRuntimeShared {
            token_next: AtomicUsize::new(0),
            tasks: Mutex::new(HashMap::new()),
        };

        let inner = ScopedRuntimeInner {
            runtime,
            shared,
            owner,
        };

        Self {
            inner: Box::pin(inner),
        }
    }

    pub fn spawn_future_detached<'s, E, R>(
        &'s self,
        executor: E,
    ) -> impl Future<Output = R> + Send + Captures<'s>
    where
        E: FnOnce(&'s O, ExitFlag) -> BoxFuture<'s, R>,
        R: Send + 'static,
    {
        let exit_flag_sender = async_flag::Sender::new();

        let future = executor(&self.inner.owner, exit_flag_sender.receiver());

        let (result_sender, result_receiver) = oneshot::channel();

        let token = self.inner.shared.token_next.fetch_add(1, Ordering::Relaxed);

        let tokio_future = {
            // SAFE: future borrow only owner, which by design of ScopedRuntime will outlive all futures
            let future = unsafe { transmute::<BoxFuture<'_, _>, BoxFuture<'static, _>>(future) };
            let shared =
                unsafe { transmute::<_, &'static ScopedRuntimeShared>(&self.inner.shared) };
            async move {
                let result: R = future.await;

                shared.tasks.lock().remove(&token);

                let _ = result_sender.send(result);
            }
        };

        let mut tasks = self.inner.shared.tasks.lock();

        let join_handle = self.inner.runtime.spawn(tokio_future);

        let task_detached = TaskDetached {
            exit_flag_sender,
            join_handle,
        };

        assert!(
            tasks.insert(token, task_detached).is_none(),
            "token duplicated"
        );

        drop(tasks);

        result_receiver.map(|result| result.unwrap())
    }

    pub fn spawn_runnables_object_detached<'s, E>(
        &'s self,
        executor: E,
    ) where
        E: FnOnce(&'s O) -> Box<[&(dyn Runnable + 's)]>,
    {
        let mut tasks = self.inner.shared.tasks.lock();

        executor(&self.inner.owner)
            .into_vec()
            .into_iter()
            .for_each(|runnable| {
                let exit_flag_sender = async_flag::Sender::new();

                let future = runnable.run(exit_flag_sender.receiver());

                let token = self.inner.shared.token_next.fetch_add(1, Ordering::Relaxed);

                let tokio_future = {
                    // SAFE: future borrow only owner, which by design of ScopedRuntime will outlive all futures
                    let future =
                        unsafe { transmute::<BoxFuture<'_, _>, BoxFuture<'static, _>>(future) };
                    let shared =
                        unsafe { transmute::<_, &'static ScopedRuntimeShared>(&self.inner.shared) };
                    async move {
                        let _: Exited = future.await;

                        shared.tasks.lock().remove(&token);
                    }
                };

                let join_handle = self.inner.runtime.spawn(tokio_future);

                let task_detached = TaskDetached {
                    exit_flag_sender,
                    join_handle,
                };

                assert!(
                    tasks.insert(token, task_detached).is_none(),
                    "token duplicated"
                );
            });

        drop(tasks);
    }

    pub fn spawn_runnable_detached<'s, E, R>(
        &'s self,
        executor: E,
    ) where
        E: FnOnce(&'s O) -> &'s R,
        R: Runnable + 's,
    {
        // TODO: Make this a bit more optimized?
        self.spawn_runnables_object_detached(|owner| Box::new([executor(owner) as &dyn Runnable]));
    }
}
impl<O> Drop for ScopedRuntime<O>
where
    O: Sync,
{
    fn drop(&mut self) {
        // Finalize all tasks
        let tasks = take(&mut *self.inner.shared.tasks.lock());
        if !tasks.is_empty() {
            self.inner.runtime.handle().block_on(async move {
                tasks
                    .into_values()
                    .map(|task| {
                        task.exit_flag_sender.signal();
                        task.join_handle
                    })
                    .collect::<JoinAll<_>>()
                    .await
                    .into_iter()
                    .for_each(|result| result.unwrap());
            });
        }
    }
}
