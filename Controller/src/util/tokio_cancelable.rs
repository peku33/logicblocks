use failure::Error;
use futures::future::Future;
use futures::future::{abortable, AbortHandle, Abortable, Aborted};
use std::thread;

pub struct ThreadedInfiniteToError {
    abort_handle: AbortHandle,
    join_handle: Option<thread::JoinHandle<()>>,
}
impl ThreadedInfiniteToError {
    pub fn new<F>(
        thread_name: String,
        future: F,
    ) -> Self
    where
        F: Future<Output = Error> + Send + 'static,
    {
        let (future_abortable, abort_handle) = abortable(future);

        let join_handle = thread::Builder::new()
            .name(thread_name)
            .spawn(move || Self::thread_main(future_abortable))
            .unwrap();

        Self {
            abort_handle,
            join_handle: Some(join_handle),
        }
    }

    fn thread_main<F>(future_abortable: Abortable<F>)
    where
        F: Future<Output = Error> + Send + 'static,
    {
        let mut runtime = tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();

        match runtime.block_on(future_abortable) {
            Ok(error) => panic!("future exited with error: {}", error),
            Err(Aborted) => (),
        };
    }
}
impl Drop for ThreadedInfiniteToError {
    fn drop(&mut self) {
        self.abort_handle.abort();
        let _ = self.join_handle.take().unwrap().join();
    }
}
