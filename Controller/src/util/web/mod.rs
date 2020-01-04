pub mod router;
pub mod sse;

use bytes::Bytes;
use failure::{format_err, Error};
use futures::future::BoxFuture;
use futures::stream::{Stream, StreamExt};
use http::{header, Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug)]
pub struct Request {
    remote_address: SocketAddr,
    http_parts: http::request::Parts,
    body: Bytes,
}
impl Request {
    pub fn new(
        remote_address: SocketAddr,
        http_parts: http::request::Parts,
        body: Bytes,
    ) -> Self {
        return Self {
            remote_address,
            http_parts,
            body,
        };
    }
    pub fn method(&self) -> &Method {
        return &self.http_parts.method;
    }

    pub fn body_parse_json<'a, T: Deserialize<'a>>(&'a self) -> Result<T, Error> {
        let content_type = self
            .http_parts
            .headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|header| header.to_str().ok());

        if content_type != Some("application/json") {
            return Err(format_err!(
                "expected content type application/json, got: {:?}",
                content_type,
            ));
        }

        let json = serde_json::from_slice(&self.body)?;

        return Ok(json);
    }

    pub fn body_parse_json_validate<'a, T: Deserialize<'a>, F: FnOnce(T) -> Result<T, Error>>(
        &'a self,
        f: F,
    ) -> Result<T, Error> {
        let value = self.body_parse_json()?;
        let value = f(value)?;
        return Ok(value);
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

    pub fn ok_empty() -> Response {
        let hyper_response = hyper::Response::builder()
            .body(hyper::Body::default())
            .unwrap();

        return Response { hyper_response };
    }
    pub fn ok_content_type_body<B>(
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
    pub fn ok_json<T: Serialize>(value: T) -> Response {
        let hyper_response = hyper::Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_vec(&value).unwrap().into())
            .unwrap();

        return Response { hyper_response };
    }
    pub fn ok_sse_stream<S: Stream<Item = sse::Event> + Sync + Send + 'static>(
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
    pub fn error_400_from_error(error: &Error) -> Response {
        let hyper_response = hyper::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(hyper::Body::from(error.to_string()))
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
