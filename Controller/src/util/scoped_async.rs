use async_trait::async_trait;
use futures::future::{abortable, AbortHandle, Aborted, BoxFuture, JoinAll};
use std::mem::transmute;
use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
};

#[async_trait]
pub trait Runnable: Send + Sync {
    async fn run(&self) -> !;
    async fn finalize(&self);
}

struct RunnableRunContext<'u> {
    runnable: &'u dyn Runnable,
    run_abort_handle: AbortHandle,
    run_join_handle: JoinHandle<Result<!, Aborted>>,
}

// Cancel + finalize() on drop
pub struct RunnableSpawnSync<'r, 'u> {
    runtime: &'r Handle,
    runnable_run_context: RunnableRunContext<'u>,
}
impl<'r, 'u> RunnableSpawnSync<'r, 'u> {
    pub fn new(
        runtime: &'r Runtime,
        runnable: &'u dyn Runnable,
    ) -> Self {
        let runtime = runtime.handle();

        // Create run() future
        let run = runnable.run();

        // Cast it to static.
        // Since we are aborting and awaiting the abort here, runtime will never outlive the future
        let run = unsafe { transmute::<BoxFuture<'u, !>, BoxFuture<'static, !>>(run) };

        // Make abortable handle
        let (run_abortable, run_abort_handle) = abortable(run);

        // Spawn
        let run_join_handle = runtime.spawn(run_abortable);

        // Keep it going
        Self {
            runtime,
            runnable_run_context: RunnableRunContext {
                runnable,
                run_abort_handle,
                run_join_handle,
            },
        }
    }
}
impl<'r, 'u> Drop for RunnableSpawnSync<'r, 'u> {
    fn drop(&mut self) {
        // Cancel running future
        self.runnable_run_context.run_abort_handle.abort();

        // Wait until its completed, check the result
        self.runtime
            .block_on(&mut self.runnable_run_context.run_join_handle)
            .unwrap()
            .expect_err("run() yielded");

        // Run blocking finalization
        self.runtime
            .block_on(&mut self.runnable_run_context.runnable.finalize());
    }
}

// Cancel + finalize() on drop for multiple runnables
// Allows concurrent finalization
pub struct RunnablesSpawnSync<'r, 'u> {
    runtime: &'r Handle,
    runnable_run_contexts: Box<[RunnableRunContext<'u>]>,
}
impl<'r, 'u> RunnablesSpawnSync<'r, 'u> {
    pub fn new(
        runtime: &'r Runtime,
        runnables: &[&'u dyn Runnable],
    ) -> Self {
        let runtime = runtime.handle();

        let runnable_run_contexts = runnables
            .iter()
            .copied()
            .map(|runnable| {
                let run = runnable.run();
                let run = unsafe { transmute::<BoxFuture<'u, !>, BoxFuture<'static, !>>(run) };
                let (run_abortable, run_abort_handle) = abortable(run);
                let run_join_handle = runtime.spawn(run_abortable);

                RunnableRunContext {
                    runnable,
                    run_abort_handle,
                    run_join_handle,
                }
            })
            .collect::<Box<[_]>>();

        Self {
            runtime,
            runnable_run_contexts,
        }
    }
}
impl<'r, 'u> Drop for RunnablesSpawnSync<'r, 'u> {
    fn drop(&mut self) {
        // Cancel running futures
        self.runnable_run_contexts
            .iter_mut()
            .for_each(|runnable_run_contexts| runnable_run_contexts.run_abort_handle.abort());

        // Wait until all are completed
        self.runtime
            .block_on(
                self.runnable_run_contexts
                    .iter_mut()
                    .map(|runnable_run_contexts| &mut runnable_run_contexts.run_join_handle)
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
