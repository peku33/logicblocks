use super::super::{Request, Response};
use super::{Handler, UriCursor};
use futures::channel::{mpsc, oneshot};
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};

struct Item {
    request: Request,
    uri_cursor: UriCursor,
    result_sender: oneshot::Sender<BoxFuture<'static, Response>>,
}
pub struct Sender {
    channel: mpsc::UnboundedSender<Item>,
}
impl Handler for Sender {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        let (result_sender, result_receiver) = oneshot::channel();

        let item = Item {
            request,
            uri_cursor,
            result_sender,
        };

        if let Err(error) = self.channel.unbounded_send(item) {
            log::error!("error while sending item: {}", error);
            return async move { Response::error_500() }.boxed();
        }

        async move {
            match result_receiver.await {
                Ok(response) => response.await,
                Err(error) => {
                    log::error!("error while receiving item: {}", error);
                    Response::error_500()
                }
            }
        }
        .boxed()
    }
}

pub struct Receiver {
    channel: mpsc::UnboundedReceiver<Item>,
}
impl Receiver {
    pub async fn attach_run(
        self,
        handler: &(dyn Handler),
    ) {
        self.channel
            .for_each(|item| async move {
                let result = handler.handle(item.request, item.uri_cursor);
                if let Err(_error) = item.result_sender.send(result) {
                    // TODO: Response needs Debug
                    log::error!("error while sending result")
                }
            })
            .await;
    }
}

pub fn channel() -> (Sender, Receiver) {
    let (sender, receiver) = mpsc::unbounded();
    (Sender { channel: sender }, Receiver { channel: receiver })
}
