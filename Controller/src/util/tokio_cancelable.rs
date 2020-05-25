use failure::Error;
use futures::{
    future::{
        abortable, AbortHandle, Abortable, Aborted, BoxFuture, FusedFuture, Future, FutureExt,
    },
    task::{Context, Poll},
};
use std::{marker::PhantomData, mem::transmute, pin::Pin, thread};

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

struct ScopedSpawnInner<R>
where
    R: Send + 'static,
{
    abort_handle: AbortHandle,
    join_handle: tokio::task::JoinHandle<Result<R, Aborted>>,
}
pub struct ScopedSpawn<'a, R>
where
    R: Send + 'static,
{
    inner: Option<ScopedSpawnInner<R>>,
    phantom_data: PhantomData<&'a R>,
}
impl<'a, R> ScopedSpawn<'a, R>
where
    R: Send + 'static,
{
    pub fn new(future: BoxFuture<'a, R>) -> Self {
        let future = unsafe { transmute::<BoxFuture<'a, R>, BoxFuture<'static, R>>(future) };
        let (abortable, abort_handle) = abortable(future);
        let join_handle = tokio::task::spawn(abortable);
        let inner = ScopedSpawnInner {
            abort_handle,
            join_handle,
        };
        Self {
            inner: Some(inner),
            phantom_data: PhantomData,
        }
    }
    pub async fn finalize(&mut self) -> Option<R> {
        let inner = self.inner.as_mut().unwrap();
        inner.abort_handle.abort();
        let result = match (&mut inner.join_handle).await.unwrap() {
            Ok(result) => Some(result),
            Err(Aborted) => None,
        };
        self.inner.take().unwrap();
        result
    }
}
impl<'a, R> Future for ScopedSpawn<'a, R>
where
    R: Send + 'static,
{
    type Output = Option<R>;
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Self::Output> {
        let self_ = unsafe { self.get_unchecked_mut() };
        let result = match self_.inner.as_mut().unwrap().join_handle.poll_unpin(cx) {
            Poll::Ready(result) => {
                let result = match result.unwrap() {
                    Ok(result) => Some(result),
                    Err(Aborted) => None,
                };
                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        };
        if result.is_ready() {
            self_.inner.take().unwrap();
        }
        result
    }
}
impl<'a, R> FusedFuture for ScopedSpawn<'a, R>
where
    R: Send + 'static,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_none()
    }
}
impl<'a, R> Drop for ScopedSpawn<'a, R>
where
    R: Send + 'static,
{
    fn drop(&mut self) {
        if self.inner.is_some() {
            panic!("ScopedSpawn should be finalize()d before dropping!");
        }
    }
}
