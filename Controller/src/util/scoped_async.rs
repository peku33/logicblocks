use super::async_flag;
use async_trait::async_trait;
use futures::future::{BoxFuture, JoinAll};
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

struct DetachedTask {
    exit_flag_sender: async_flag::Sender,
    join_handle: JoinHandle<()>,
}
struct ScopedRuntimeInner {
    token_next: AtomicUsize,
    tasks: Mutex<HashMap<usize, DetachedTask>>,
}
pub struct ScopedRuntime<O>
where
    O: Sync,
{
    runtime: Runtime,
    owner: Pin<Box<O>>,                  // Stable address
    inner: Pin<Box<ScopedRuntimeInner>>, // Stable address
}
impl<O> ScopedRuntime<O>
where
    O: Sync,
{
    pub fn new(
        owner: O,
        thread_name: String,
    ) -> Self {
        let runtime = RuntimeBuilder::new()
            .enable_all()
            .threaded_scheduler()
            .thread_name(thread_name)
            .build()
            .unwrap();

        let inner = ScopedRuntimeInner {
            token_next: AtomicUsize::new(0),
            tasks: Mutex::new(HashMap::new()),
        };
        let inner = Box::pin(inner);

        Self {
            runtime,
            owner: Box::pin(owner),
            inner,
        }
    }

    // pub fn spawn_future<'s, E, R>(
    //     &'s self,
    //     executor: E,
    // ) -> impl Future<Output = R> + Send + Captures<'s>
    // where
    //     E: FnOnce(&'s O, ExitFlag) -> BoxFuture<'s, R>,
    //     R: Send + 'static,
    // {
    //     // Prepare token
    //     let token = self.inner.token_next.fetch_add(1, Ordering::Relaxed);

    //     // Prepare exit flag
    //     let exit_flag_sender = async_flag::Sender::new();
    //     let exit_flag = exit_flag_sender.receiver();

    //     // Prepare result sending channel
    //     let (result_sender, result_receiver) = oneshot::channel();

    //     // Spawn future
    //     let future = executor(&self.owner, exit_flag);

    //     // SAFE: future borrow &self.owner which is StableAddress and guaranteed not to drop before this future
    //     let future = unsafe { transmute::<BoxFuture<'s, R>, BoxFuture<'static, R>>(future) };
    //     let inner = unsafe { transmute::<_, &'static ScopedRuntimeInner>(&*self.inner) };

    //     // Spawn spawnable future
    //     let tokio_future = async move {
    //         // Wait until future is ready
    //         let result = future.await;

    //         // Publish the result
    //         // This may fail if caller dropped the receiver, but we don't care
    //         let _ = result_sender.send(result);

    //         // Remove from pending task list
    //         let mut tasks = inner.tasks.lock();
    //         tasks.remove(&token); // This could be none if drop() takes tasks
    //     };

    //     // Keep tasks locked before spawn(), otherwise future may complete before its registered
    //     {
    //         let mut tasks = self.inner.tasks.lock();

    //         // Start the actual task
    //         let join_handle = self.runtime.spawn(tokio_future);

    //         // Create DetachedTask
    //         let detached_task = DetachedTask {
    //             exit_flag_sender,
    //             join_handle,
    //         };

    //         // Register it in pending list
    //         assert!(
    //             tasks.insert(token, detached_task).is_none(),
    //             "token duplicated"
    //         );
    //     }

    //     // Returned future is + 's, so it must be dropped before canceling the future in this class Drop
    //     result_receiver.map(|result| result.unwrap())
    // }

    pub fn spawn_runnables_object_detached<'s, E>(
        &'s self,
        executor: E,
    ) where
        E: FnOnce(&'s O) -> Box<[&(dyn Runnable + 's)]>,
    {
        let mut tasks = self.inner.tasks.lock();

        executor(&self.owner)
            .into_vec()
            .into_iter()
            .for_each(|runnable| {
                let exit_flag_sender = async_flag::Sender::new();

                let future = runnable.run(exit_flag_sender.receiver());
                // SAFE: future borrow only owner, which by design of ScopedRuntime will outlive all futures
                let future =
                    unsafe { transmute::<BoxFuture<'_, _>, BoxFuture<'static, _>>(future) };

                // SAFE: As above
                let inner = unsafe { transmute::<_, &'static ScopedRuntimeInner>(&*self.inner) };

                let token = self.inner.token_next.fetch_add(1, Ordering::Relaxed);

                let tokio_future = async move {
                    // Wait until future is ready
                    let _: Exited = future.await;

                    // Remove from pending task list
                    let mut tasks = inner.tasks.lock();
                    tasks.remove(&token); // This could be none if drop() takes tasks
                };

                let join_handle = self.runtime.spawn(tokio_future);

                // Create DetachedTask
                let detached_task = DetachedTask {
                    exit_flag_sender,
                    join_handle,
                };

                // Register it in pending list
                assert!(
                    tasks.insert(token, detached_task).is_none(),
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
        self.spawn_runnables_object_detached(|owner| Box::new([executor(owner) as &dyn Runnable]));
    }

    // Finalizes all scoped tasks in asynchronous way.
    // This must be called before dropping this if drop is used from async context
    pub async fn finalize(&mut self) {
        // Clear tasks structure to release the mutex
        let tasks = take(&mut *self.inner.tasks.lock());

        // Drop early if nothing to do
        if tasks.is_empty() {
            return;
        }

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
    }
}
impl<O> Drop for ScopedRuntime<O>
where
    O: Sync,
{
    fn drop(&mut self) {
        // Clear tasks structure to release the mutex
        let tasks = take(&mut *self.inner.tasks.lock());

        // Drop early if nothing to do, to avoid block_on in async context
        if tasks.is_empty() {
            return;
        }

        // Await all pending tasks
        self.runtime.handle().block_on(async move {
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
