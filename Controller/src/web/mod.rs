pub mod router;
pub mod server;
pub mod sse;

use failure::{err_msg, Error};
use futures::future::{ready, BoxFuture, FutureExt};
use futures::stream::{Stream, StreamExt};
use http::{header, Method, StatusCode};
use serde_json::Value as JsonValue;
use std::net::SocketAddr;

#[derive(Debug)]
pub struct Request {
    remote_address: SocketAddr,
    hyper_request: hyper::Request<hyper::Body>,
}
impl Request {
    pub fn new(
        remote_address: SocketAddr,
        hyper_request: hyper::Request<hyper::Body>,
    ) -> Self {
        return Self {
            remote_address,
            hyper_request,
        };
    }
    pub fn method(&self) -> &Method {
        return self.hyper_request.method();
    }
}

#[derive(Debug)]
pub struct Response {
    hyper_response: hyper::Response<hyper::Body>,
}
impl Response {
    pub fn into_hyper_response(self) -> hyper::Response<hyper::Body> {
        return self.hyper_response;
    }

    pub fn from_body_content_type<B>(
        body: B,
        content_type: &str,
    ) -> Response
    where
        B: Into<hyper::Body>,
    {
        let hyper_response = hyper::Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .body(body.into())
            .unwrap();

        return Response { hyper_response };
    }

    pub fn from_json(value: JsonValue) -> Response {
        let hyper_response = hyper::Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_vec(&value).unwrap().into())
            .unwrap();

        return Response { hyper_response };
    }

    pub fn from_sse_stream<S: Stream<Item = sse::Event> + Sync + Send + 'static>(
        sse_stream: S
    ) -> Response {
        let hyper_body = hyper::Body::wrap_stream(sse_stream.map(|event| {
            return Ok::<_, Error>(event.serialize());
        }));
        let hyper_response = hyper::Response::builder()
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(hyper_body)
            .unwrap();
        return Response { hyper_response };
    }

    pub fn error(status_code: StatusCode) -> Response {
        let hyper_response = hyper::Response::builder()
            .status(status_code)
            .body(hyper::Body::default())
            .unwrap();

        return Response { hyper_response };
    }
    pub fn error_404() -> Response {
        return Self::error(StatusCode::NOT_FOUND);
    }
}

pub trait Handler {
    fn handle(
        &self,
        request: &Request,
    ) -> BoxFuture<'static, Response>;
}
pub trait HandlerThreaded: Sync + Send {
    fn handle(
        &self,
        request: Box<Request>,
    ) -> BoxFuture<'static, Response>;
}

pub fn handler_async<'r>(
    target: &'r dyn Handler
) -> (HandlerAsyncSender, HandlerAsyncReceiver<'r>) {
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    let sender = HandlerAsyncSender { sender };
    let receiver = HandlerAsyncReceiver { target, receiver };
    return (sender, receiver);
}
struct HandlerAsyncItem {
    request: Box<Request>,
    response_channel: tokio::sync::oneshot::Sender<BoxFuture<'static, Response>>,
}
pub struct HandlerAsyncReceiver<'r> {
    target: &'r dyn Handler,
    receiver: futures::channel::mpsc::UnboundedReceiver<HandlerAsyncItem>,
}
impl<'r> HandlerAsyncReceiver<'r> {
    pub async fn run(self) -> Error {
        let target = self.target;
        self.receiver
            .for_each(|handler_async_item| {
                let response_future = target.handle(&handler_async_item.request);
                let send_result = handler_async_item.response_channel.send(response_future);
                if send_result.is_err() {
                    panic!("response_channel.send(response_future) paniced");
                }
                return ready(());
            })
            .await;
        return err_msg("for_each() exited");
    }
}

pub struct HandlerAsyncSender {
    sender: futures::channel::mpsc::UnboundedSender<HandlerAsyncItem>,
}
impl HandlerThreaded for HandlerAsyncSender {
    fn handle(
        &self,
        request: Box<Request>,
    ) -> BoxFuture<'static, Response> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let handler_async_receiver_item = HandlerAsyncItem {
            request: request,
            response_channel: sender,
        };
        let send_result = self
            .sender
            .clone()
            .unbounded_send(handler_async_receiver_item);
        if send_result.is_err() {
            panic!("try_send(handler_async_receiver_item) paniced");
        }
        return async move {
            let response_future = receiver.await.unwrap();
            let response = response_future.await;
            return response;
        }
        .boxed();
    }
}