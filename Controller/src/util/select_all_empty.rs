use futures::future::{FusedFuture, Future, FutureExt};
use futures::stream::{FuturesUnordered, Stream};
use futures::task::{Context, Poll};
use std::iter::FromIterator;
use std::marker::Unpin;
use std::pin::Pin;

pub struct SelectAllEmptyFuture<F: Future + Unpin> {
    futures: Box<[F]>,
}
impl<F: Future + Unpin> FromIterator<F> for SelectAllEmptyFuture<F> {
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        Self {
            futures: iter.into_iter().collect(),
        }
    }
}
impl<F: Future + Unpin> Future for SelectAllEmptyFuture<F> {
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

pub struct SelectAllEmptyFutureInfinite<F: Future<Output = !>> {
    futures_unordered: FuturesUnordered<F>,
}
impl<F: Future<Output = !>> FromIterator<F> for SelectAllEmptyFutureInfinite<F> {
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        Self {
            futures_unordered: FuturesUnordered::from_iter(iter),
        }
    }
}
impl<F: Future<Output = !>> Future for SelectAllEmptyFutureInfinite<F> {
    type Output = !;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Self::Output> {
        let futures_unordered =
            unsafe { self.map_unchecked_mut(|self_| &mut self_.futures_unordered) };
        match futures_unordered.len() {
            0 => Poll::Pending,
            _ => match futures_unordered.poll_next(cx) {
                Poll::Ready(Some(_)) => panic!("futures_unordered yielded"),
                Poll::Ready(None) => panic!("futures_unordered completed"),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}
impl<F: Future<Output = !>> FusedFuture for SelectAllEmptyFutureInfinite<F> {
    fn is_terminated(&self) -> bool {
        false
    }
}
