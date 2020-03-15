use futures::future::{Future, FutureExt};
use futures::task::{Context, Poll};
use std::marker::Unpin;
use std::pin::Pin;

pub fn select_all_empty<I>(iterator: I) -> SelectAllEmpty<I::Item>
where
    I: IntoIterator,
    I::Item: Future + Unpin,
{
    SelectAllEmpty {
        futures: iterator.into_iter().collect(),
    }
}

pub struct SelectAllEmpty<F: Future + Unpin> {
    futures: Vec<F>,
}
impl<F: Future + Unpin> Future for SelectAllEmpty<F> {
    type Output = (F::Output, usize);

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Self::Output> {
        for (id, future) in self.get_mut().futures.iter_mut().enumerate() {
            if let Poll::Ready(result) = future.poll_unpin(cx) {
                return Poll::Ready((result, id));
            }
        }
        Poll::Pending
    }
}
