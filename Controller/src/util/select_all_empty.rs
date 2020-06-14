use futures::{
    future::{FusedFuture, Future},
    stream::{FuturesUnordered, Stream},
    task::{Context, Poll},
};
use std::{iter::FromIterator, pin::Pin};

pub struct JoinAllEmptyUnit<F: Future<Output = ()>> {
    futures_unordered: FuturesUnordered<F>,
}
impl<F: Future<Output = ()>> FromIterator<F> for JoinAllEmptyUnit<F> {
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        Self {
            futures_unordered: FuturesUnordered::from_iter(iter),
        }
    }
}
impl<F: Future<Output = ()>> Future for JoinAllEmptyUnit<F> {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Self::Output> {
        let futures_unordered =
            unsafe { self.map_unchecked_mut(|self_| &mut self_.futures_unordered) };
        match futures_unordered.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(_)) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(None) => Poll::Ready(()),
        }
    }
}
impl<F: Future<Output = ()>> FusedFuture for JoinAllEmptyUnit<F> {
    fn is_terminated(&self) -> bool {
        self.futures_unordered.is_empty()
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
