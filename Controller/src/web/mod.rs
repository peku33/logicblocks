pub mod root_service;
pub mod server;
pub mod sse;
pub mod sse_topic;
pub mod uri_cursor;

use anyhow::{ensure, Context, Error};
use bytes::Bytes;
use futures::{
    future::BoxFuture,
    stream::{once, Stream, StreamExt},
};
use http::{header, request::Parts, HeaderMap, HeaderValue, Method, StatusCode, Uri};
use hyper::{Body, Response as HyperResponse};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug)]
pub struct Request {
    remote_address: SocketAddr,
    http_parts: Parts,
    body: Bytes,
}
impl Request {
    pub fn new(
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
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.http_parts.headers
    }

    pub fn body_parse_json<'a, T: Deserialize<'a>>(&'a self) -> Result<T, Error> {
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

#[derive(Debug)]
pub struct Response {
    hyper_response: HyperResponse<Body>,
}
impl Response {
    pub fn from_hyper_response(hyper_response: HyperResponse<Body>) -> Self {
        Self { hyper_response }
    }
    pub fn into_hyper_response(self) -> HyperResponse<Body> {
        self.hyper_response
    }

    pub fn status_code(&self) -> StatusCode {
        self.hyper_response.status()
    }

    pub fn ok_empty() -> Self {
        let hyper_response = HyperResponse::builder().body(Body::default()).unwrap();

        Response { hyper_response }
    }
    pub fn ok_content_type_body<B>(
        content_type: &str,
        body: B,
    ) -> Self
    where
        B: Into<Body>,
    {
        let hyper_response = HyperResponse::builder()
            .header(header::CONTENT_TYPE, content_type)
            .body(body.into())
            .unwrap();

        Response { hyper_response }
    }
    pub fn ok_json<T: Serialize>(value: T) -> Self {
        let body = serde_json::to_vec(&value).unwrap();
        let hyper_response = HyperResponse::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        Response { hyper_response }
    }
    pub fn ok_sse_stream<S: Stream<Item = sse::Event> + Send + 'static>(sse_stream: S) -> Self {
        // FIXME: webkit based browsers (firefox, safari) won't see the stream opened until something is written
        let ping_event = ":\r\n".to_owned();
        let hyper_body = Body::wrap_stream(
            once(async { Ok(ping_event) })
                .chain(sse_stream.map(|event| Ok::<_, Error>(event.to_payload()))),
        );
        let hyper_response = HyperResponse::builder()
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(hyper_body)
            .unwrap();
        Response { hyper_response }
    }

    pub fn redirect_302(target: &str) -> Self {
        let hyper_response = HyperResponse::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, target)
            .body(Body::default())
            .unwrap();
        Response { hyper_response }
    }

    pub fn error(status_code: StatusCode) -> Self {
        let hyper_response = HyperResponse::builder()
            .status(status_code)
            .body(Body::default())
            .unwrap();

        Response { hyper_response }
    }
    pub fn error_400_from_error<T: Into<Error>>(error: T) -> Self {
        let hyper_response = HyperResponse::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(error.into().to_string()))
            .unwrap();
        Response { hyper_response }
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
