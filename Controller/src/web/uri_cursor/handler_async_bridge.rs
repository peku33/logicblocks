use super::{
    super::{Request, Response},
    Handler, HandlerAsync, UriCursor,
};
use crate::util::atomic_cell::AtomicCell;
use futures::{
    channel::{mpsc, oneshot},
    future::{BoxFuture, FutureExt},
    stream::StreamExt,
};

struct HandlerAsyncBridgeItem {
    request: Request,
    uri_cursor: &UriCursor,
    result_future_sender: oneshot::Sender<BoxFuture<'static, Response>>,
}

pub struct HandlerAsyncBridge {
    requests_sender: mpsc::UnboundedSender<HandlerAsyncBridgeItem>,
    requests_receiver: AtomicCell<mpsc::UnboundedReceiver<HandlerAsyncBridgeItem>>,
}
impl HandlerAsyncBridge {
    pub fn new() -> Self {
        let (requests_sender, requests_receiver) = mpsc::unbounded();
        let requests_receiver = AtomicCell::new(requests_receiver);

        Self {
            requests_sender,
            requests_receiver,
        }
    }
    pub async fn run<H: HandlerAsync + Sync>(
        &self,
        handler_async: &H,
    ) -> ! {
        let mut requests_receiver = self.requests_receiver.lease();
        requests_receiver
            .by_ref()
            .for_each(async move |item| {
                let result_future = handler_async.handle(item.request, item.uri_cursor).await;

                // Error here means receiver was dropped before receiving
                match item.result_future_sender.send(result_future) {
                    Ok(()) => {}
                    Err(_) => {
                        log::warn!("result_future_receiver was dropped before receiving result");
                    }
                }
            })
            .await;
        panic!("requests_receiver yielded");
    }
}
impl Handler for HandlerAsyncBridge {
    fn handle(
        &self,
        request: Request,
        uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        let (result_future_sender, result_future_receiver) = oneshot::channel();

        self.requests_sender
            .unbounded_send(HandlerAsyncBridgeItem {
                request,
                uri_cursor,
                result_future_sender,
            })
            .unwrap();

        async move {
            let result_future = match result_future_receiver.await {
                Ok(result_future) => result_future,
                Err(_) => async move {
                    log::warn!("result_future closed before finishing");
                    Response::error_500()
                }
                .boxed(),
            };
            result_future.await
        }
        .boxed()
    }
}
