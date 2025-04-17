pub mod root_service;
pub mod server;
pub mod sse;
pub mod sse_topic;
pub mod uri_cursor;

use anyhow::{Context, Error, ensure};
use bytes::Bytes;
use futures::{
    future::BoxFuture,
    stream::{Stream, StreamExt, once},
};
use http::{HeaderMap, Method, Response as HttpResponse, StatusCode, Uri, header, request::Parts};
use http_body_util::{BodyExt, Empty, Full, StreamBody, combinators::BoxBody};
use hyper::body::Frame;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, net::SocketAddr};

#[derive(Debug)]
pub struct Request {
    remote_address: SocketAddr,
    http_parts: Parts,
    body_payload: Bytes,
}
impl Request {
    pub fn from_http_request(
        remote_address: SocketAddr,
        http_parts: Parts,
        body_payload: Bytes,
    ) -> Self {
        Self {
            remote_address,
            http_parts,
            body_payload,
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

        let json = serde_json::from_slice(&self.body_payload).context("from_slice")?;

        Ok(json)
    }
}

#[derive(Debug)]
pub struct Response {
    http_response: HttpResponse<BoxBody<Bytes, Infallible>>,
}
impl Response {
    pub fn from_http_response(http_response: HttpResponse<BoxBody<Bytes, Infallible>>) -> Self {
        Self { http_response }
    }
    pub fn into_http_response(self) -> HttpResponse<BoxBody<Bytes, Infallible>> {
        self.http_response
    }

    pub fn status_code(&self) -> StatusCode {
        self.http_response.status()
    }

    pub fn ok_empty() -> Self {
        let http_response = HttpResponse::builder().body(Empty::new().boxed()).unwrap();

        Self { http_response }
    }
    pub fn ok_content_type_body(
        content_type: &str,
        body_payload: Bytes,
    ) -> Self {
        let http_response = HttpResponse::builder()
            .header(header::CONTENT_TYPE, content_type)
            .body(Full::new(body_payload).boxed())
            .unwrap();

        Self { http_response }
    }
    pub fn ok_json<T: Serialize>(value: T) -> Self {
        let body_payload = Bytes::from(serde_json::to_vec(&value).unwrap());

        let http_response = HttpResponse::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(body_payload).boxed())
            .unwrap();

        Self { http_response }
    }
    pub fn ok_sse_stream<S: Stream<Item = sse::Event> + Send + Sync + 'static>(
        sse_stream: S
    ) -> Self {
        // FIXME: webkit based browsers (firefox, safari) won't see the stream opened
        // until something is written
        // FIXME: break stream on app exit
        let ping_event = ":\r\n".to_owned();
        let body_payload_frame_stream = once(async move { ping_event })
            .chain(sse_stream.map(|event| event.to_payload()))
            .map(|payload| Frame::data(Bytes::from(payload)));

        let http_response = HttpResponse::builder()
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(BodyExt::boxed(StreamBody::new(
                body_payload_frame_stream.map(Ok),
            )))
            .unwrap();
        Self { http_response }
    }

    pub fn redirect_302(target: &str) -> Self {
        let http_response = HttpResponse::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, target)
            .body(Empty::new().boxed())
            .unwrap();
        Self { http_response }
    }

    pub fn error(status_code: StatusCode) -> Self {
        let http_response = HttpResponse::builder()
            .status(status_code)
            .body(Empty::new().boxed())
            .unwrap();

        Self { http_response }
    }
    pub fn error_400_from_error<T: Into<Error>>(error: T) -> Self {
        let body_payload = Bytes::from(error.into().to_string());
        let http_response = HttpResponse::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(body_payload).boxed())
            .unwrap();
        Self { http_response }
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
}

pub trait Handler {
    fn handle(
        &self,
        request: Request,
    ) -> BoxFuture<'static, Response>;
}
