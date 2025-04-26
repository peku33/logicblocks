pub mod root_service;
pub mod server;
pub mod sse;
pub mod sse_topic;
pub mod uri_cursor;

use crate::util::{
    async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt, async_flag,
};
use anyhow::{Context, Error, ensure};
use bytes::Bytes;
use futures::{
    future::BoxFuture,
    stream::{BoxStream, Stream, StreamExt, once},
};
use http::{HeaderMap, Method, Response as HttpResponse, StatusCode, Uri, header, request::Parts};
use http_body_util::{BodyExt, Empty, Full, StreamBody, combinators::UnsyncBoxBody};
use hyper::body::{Body, Frame};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, convert::Infallible, net::SocketAddr};

#[derive(Debug)]
pub struct Request {
    remote_address: SocketAddr,
    http_parts: Parts,
    body: Bytes,
}
impl Request {
    pub fn from_http_request(
        remote_address: SocketAddr,
        http_parts: Parts,
        body: Bytes,
    ) -> Self {
        Self {
            remote_address,
            http_parts,
            body,
        }
    }

    pub fn method(&self) -> &Method {
        &self.http_parts.method
    }
    pub fn uri(&self) -> &Uri {
        &self.http_parts.uri
    }
    pub fn headers(&self) -> &HeaderMap {
        &self.http_parts.headers
    }

    pub fn body_parse_json<'s, T: Deserialize<'s>>(&'s self) -> Result<T, Error> {
        let content_type = self
            .http_parts
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|header| header.to_str().ok());

        ensure!(
            content_type == Some("application/json"),
            "expected content type application/json, got: {:?}",
            content_type,
        );

        let json = serde_json::from_slice(&self.body).context("from_slice")?;

        Ok(json)
    }
}

#[derive(Debug, derive_more::From)]
pub enum Response {
    Empty(ResponseEmpty),
    Full(ResponseFull),
    SseStream(ResponseSseStream),
    Redirect(ResponseRedirect),
    Wrapping(HttpResponse<UnsyncBoxBody<Bytes, Infallible>>),
}
impl Response {
    pub fn ok_empty() -> Self {
        Self::from(ResponseEmpty::ok())
    }
    pub fn ok_json<T: Serialize>(value: T) -> Self {
        Self::from(ResponseFull::ok_json(value))
    }
    pub fn ok_content_type_body(
        content_type: Cow<'static, str>,
        body: Bytes,
    ) -> Self {
        Self::from(ResponseFull::ok_content_type_body(content_type, body))
    }
    pub fn ok_sse_stream<S: Stream<Item = sse::Event> + Send + 'static>(stream: S) -> Self {
        Self::from(ResponseSseStream::ok(stream))
    }

    pub fn redirect(target: Cow<'static, str>) -> Self {
        Self::from(ResponseRedirect::redirect(target))
    }

    pub fn error_400_from_error<T: Into<Error>>(error: T) -> Self {
        Self::from(ResponseFull::error_400_from_error(error))
    }
    pub fn error_404() -> Self {
        Self::from(ResponseEmpty::error_404())
    }
    pub fn error_405() -> Self {
        Self::from(ResponseEmpty::error_405())
    }

    pub fn into_http_response(
        self,
        exit_flag_template: &async_flag::Receiver,
    ) -> HttpResponse<UnsyncBoxBody<Bytes, Infallible>> {
        match self {
            Response::Empty(response_empty) => response_empty
                .into_http_response()
                .map(|body| body.boxed_unsync()),
            Response::Full(response_full) => response_full
                .into_http_response()
                .map(|body| body.boxed_unsync()),
            Response::SseStream(response_sse_stream) => {
                let exit_flag = exit_flag_template.clone();
                response_sse_stream
                    .into_http_response(exit_flag)
                    .map(|body| body.boxed_unsync())
            }
            Response::Redirect(response_redirect) => response_redirect
                .into_http_response()
                .map(|body| body.boxed_unsync()),
            Response::Wrapping(response) => response,
        }
    }
}

#[derive(Debug)]
pub struct ResponseEmpty {
    status_code: StatusCode,
}
impl ResponseEmpty {
    pub fn ok() -> Self {
        let status_code = StatusCode::OK;

        Self { status_code }
    }

    pub fn error(status_code: StatusCode) -> Self {
        Self { status_code }
    }
    pub fn error_404() -> Self {
        Self::error(StatusCode::NOT_FOUND)
    }
    pub fn error_405() -> Self {
        Self::error(StatusCode::METHOD_NOT_ALLOWED)
    }
    pub fn error_500() -> Self {
        Self::error(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn into_http_response(self) -> HttpResponse<Empty<Bytes>> {
        let http_response = HttpResponse::builder()
            .status(self.status_code)
            .body(Empty::new())
            .unwrap();

        http_response
    }
}

#[derive(Debug)]
pub struct ResponseFull {
    status_code: StatusCode,
    content_type: Cow<'static, str>,
    body: Bytes,
}
impl ResponseFull {
    pub fn ok_json<T: Serialize>(value: T) -> Self {
        let status_code = StatusCode::OK;
        let body = Bytes::from(serde_json::to_vec(&value).unwrap());
        let content_type = Cow::from("application/json");

        Self {
            status_code,
            content_type,
            body,
        }
    }
    pub fn ok_content_type_body(
        content_type: Cow<'static, str>,
        body: Bytes,
    ) -> Self {
        let status_code = StatusCode::OK;

        Self {
            status_code,
            content_type,
            body,
        }
    }
    pub fn error_400_from_error<T: Into<Error>>(error: T) -> Self {
        let status_code = StatusCode::BAD_REQUEST;
        let content_type = Cow::from("text/plain");
        let body = Bytes::from(error.into().to_string());

        Self {
            status_code,
            content_type,
            body,
        }
    }

    pub fn into_http_response(self) -> HttpResponse<Full<Bytes>> {
        let http_response = HttpResponse::builder()
            .status(self.status_code)
            .header(header::CONTENT_TYPE, &*self.content_type)
            .body(Full::new(self.body))
            .unwrap();

        http_response
    }
}

#[derive(derive_more::Debug)]
pub struct ResponseSseStream {
    #[debug(skip)]
    inner: BoxStream<'static, sse::Event>,
}
impl ResponseSseStream {
    pub fn ok<S: Stream<Item = sse::Event> + Send + 'static>(inner: S) -> Self {
        let inner = inner.boxed();

        Self { inner }
    }
    pub fn into_http_response(
        self,
        exit_flag: async_flag::Receiver,
    ) -> HttpResponse<impl Body<Data = Bytes, Error = Infallible>> {
        // NOTE: webkit based browsers (firefox, safari) won't see the stream opened
        // until something is written
        let ping_event = ":\r\n".to_owned();
        let body_frame_stream = once(async move { ping_event })
            .chain(self.inner.map(|event| event.to_payload()))
            .stream_take_until_exhausted(exit_flag)
            .map(|payload| Frame::data(Bytes::from(payload)))
            .map(Ok::<_, Infallible>);

        let http_response = HttpResponse::builder()
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(StreamBody::new(body_frame_stream))
            .unwrap();

        http_response
    }
}

#[derive(Debug)]
pub struct ResponseRedirect {
    target: Cow<'static, str>,
}
impl ResponseRedirect {
    pub fn redirect(target: Cow<'static, str>) -> Self {
        Self { target }
    }

    pub fn into_http_response(self) -> HttpResponse<Empty<Bytes>> {
        let http_response = HttpResponse::builder()
            .status(StatusCode::TEMPORARY_REDIRECT)
            .header(header::LOCATION, &*self.target)
            .body(Empty::new())
            .unwrap();

        http_response
    }
}

pub trait Handler {
    fn handle(
        &self,
        request: Request,
    ) -> BoxFuture<'static, Response>;
}
