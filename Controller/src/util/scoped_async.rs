use async_trait::async_trait;
use futures::future::{abortable, AbortHandle, Aborted, BoxFuture, JoinAll};
use std::mem::transmute;
use tokio::{runtime::Handle, task::JoinHandle};

#[async_trait]
pub trait Runnable: Send + Sync {
    async fn run(&self) -> !;
    async fn finalize(&self);
}

struct RunnableRunContext<'u> {
    runnable: &'u dyn Runnable,
    run_future_abort_handle: AbortHandle,
    run_future_join_handle: JoinHandle<Result<!, Aborted>>,
}

// Cancel + finalize() on drop
pub struct ScopedRunnerSync<'r, 'u> {
    runtime: &'r Handle,
    runnable_run_context: RunnableRunContext<'u>,
}
impl<'r, 'u> ScopedRunnerSync<'r, 'u> {
    pub fn new(
        runtime: &'r Handle,
        runnable: &'u dyn Runnable,
    ) -> Self {
        // Create run() future
        let run_future = runnable.run();

        // Cast it to static.
        // Since we are aborting and awaiting the abort here, runtime will never outlive the future
        let run_future =
            unsafe { transmute::<BoxFuture<'u, !>, BoxFuture<'static, !>>(run_future) };

        // Make abortable handle
        let (run_future_abortable, run_future_abort_handle) = abortable(run_future);

        // Spawn
        let run_future_join_handle = runtime.spawn(run_future_abortable);

        // Keep it going
        Self {
            runtime,
            runnable_run_context: RunnableRunContext {
                runnable,
                run_future_abort_handle,
                run_future_join_handle,
            },
        }
    }
}
impl<'r, 'u> Drop for ScopedRunnerSync<'r, 'u> {
    fn drop(&mut self) {
        // Cancel running future
        self.runnable_run_context.run_future_abort_handle.abort();

        // Wait until its completed, check the result
        self.runtime
            .block_on(&mut self.runnable_run_context.run_future_join_handle)
            .unwrap()
            .expect_err("run() yielded");

        // Run blocking finalization
        self.runtime
            .block_on(&mut self.runnable_run_context.runnable.finalize());
    }
}

// Cancel + finalize() on drop for multiple runnables
// Allows concurrent finalization
pub struct ScopedRunnersSync<'r, 'u> {
    runtime: &'r Handle,
    runnable_run_contexts: Box<[RunnableRunContext<'u>]>,
}
impl<'r, 'u> ScopedRunnersSync<'r, 'u> {
    pub fn new(
        runtime: &'r Handle,
        runnables: &[&'u dyn Runnable],
    ) -> Self {
        let runnable_run_contexts = runnables
            .iter()
            .copied()
            .map(|runnable| {
                let run_future = runnable.run();
                let run_future =
                    unsafe { transmute::<BoxFuture<'u, !>, BoxFuture<'static, !>>(run_future) };
                let (run_future_abortable, run_future_abort_handle) = abortable(run_future);
                let run_future_join_handle = runtime.spawn(run_future_abortable);

                RunnableRunContext {
                    runnable,
                    run_future_abort_handle,
                    run_future_join_handle,
                }
            })
            .collect::<Box<[_]>>();

        Self {
            runtime,
            runnable_run_contexts,
        }
    }
}
impl<'r, 'u> Drop for ScopedRunnersSync<'r, 'u> {
    fn drop(&mut self) {
        // Cancel running futures
        self.runnable_run_contexts
            .iter_mut()
            .for_each(|runnable_run_contexts| {
                runnable_run_contexts.run_future_abort_handle.abort()
            });

        // Wait until all are completed
        self.runtime
            .block_on(
                self.runnable_run_contexts
                    .iter_mut()
                    .map(|runnable_run_contexts| &mut runnable_run_contexts.run_future_join_handle)
                    .collect::<JoinAll<_>>(),
            )
            .into_iter()
            .for_each(|result| {
                result.unwrap().expect_err("run() yielded");
            });

        // Run blocking finalization
        self.runtime.block_on(
            self.runnable_run_contexts
                .iter_mut()
                .map(|runnable_run_contexts| runnable_run_contexts.runnable.finalize())
                .collect::<JoinAll<_>>(),
        );
    }
}
