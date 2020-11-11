use futures::{
    future::{abortable, AbortHandle, Aborted, BoxFuture, FusedFuture, Future, FutureExt},
    task::{Context, Poll},
};
use std::{marker::PhantomData, mem::transmute, pin::Pin};
use tokio::{runtime::Runtime, task::JoinHandle};

struct ScopedSpawnInner<R>
where
    R: Send + 'static,
{
    abort_handle: AbortHandle,
    join_handle: JoinHandle<Result<R, Aborted>>,
}
pub struct ScopedSpawn<'r, 'a, R>
where
    R: Send + 'static,
{
    runtime: &'r Runtime,
    inner: Option<ScopedSpawnInner<R>>,
    phantom_future: PhantomData<&'a R>,
}
impl<'r, 'a, R> ScopedSpawn<'r, 'a, R>
where
    R: Send + 'static,
{
    pub fn new(
        runtime: &'r Runtime,
        future: BoxFuture<'a, R>,
    ) -> Self {
        let future = unsafe { transmute::<BoxFuture<'a, R>, BoxFuture<'static, R>>(future) };
        let (abortable, abort_handle) = abortable(future);
        let join_handle = runtime.spawn(abortable);
        let inner = ScopedSpawnInner {
            abort_handle,
            join_handle,
        };
        Self {
            runtime,
            inner: Some(inner),
            phantom_future: PhantomData,
        }
    }
    pub async fn abort(&mut self) -> Option<R> {
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
impl<'r, 'a, R> Future for ScopedSpawn<'r, 'a, R>
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
impl<'r, 'a, R> FusedFuture for ScopedSpawn<'r, 'a, R>
where
    R: Send + 'static,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_none()
    }
}
impl<'r, 'a, R> Drop for ScopedSpawn<'r, 'a, R>
where
    R: Send + 'static,
{
    fn drop(&mut self) {
        if self.inner.is_some() {
            panic!("ScopedSpawn should be aborted() before dropping!");
        }
    }
}
