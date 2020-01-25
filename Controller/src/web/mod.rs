pub mod hyper_body_variant;
pub mod root_service;
pub mod server;
pub mod sse;
pub mod uri_cursor;

use bytes::Bytes;
use failure::{format_err, Error};
use futures::future::BoxFuture;
use futures::stream::{Stream, StreamExt};
use http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri};
use hyper_body_variant::HttpBodyVariant;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

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

        if content_type != Some("application/json") {
            return Err(format_err!(
                "expected content type application/json, got: {:?}",
                content_type,
            ));
        }

        let json = serde_json::from_slice(&self.body)?;

        Ok(json)
    }

    pub fn body_parse_json_validate<'a, T: Deserialize<'a>, F: FnOnce(T) -> Result<T, Error>>(
        &'a self,
        f: F,
    ) -> Result<T, Error> {
        let value = self.body_parse_json()?;
        let value = f(value)?;
        Ok(value)
    }
}

pub struct Response {
    hyper_response: hyper::Response<HttpBodyVariant>,
}
impl Response {
    pub fn into_hyper_response(self) -> hyper::Response<HttpBodyVariant> {
        self.hyper_response
    }

    pub fn wrap_web_static_pack_response(
        response: hyper::Response<web_static_pack::hyper_loader::StaticBody>
    ) -> Response {
        let (parts, body) = response.into_parts();
        Response {
            hyper_response: hyper::Response::from_parts(parts, HttpBodyVariant::from(body)),
        }
    }

    pub fn ok_empty() -> Response {
        let hyper_response = hyper::Response::builder()
            .body(HttpBodyVariant::from(hyper::Body::default()))
            .unwrap();

        Response { hyper_response }
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
            .body(HttpBodyVariant::from(body.into()))
            .unwrap();

        Response { hyper_response }
    }
    pub fn ok_json<T: Serialize>(value: T) -> Response {
        let hyper_response = hyper::Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(HttpBodyVariant::from(hyper::Body::from(
                serde_json::to_vec(&value).unwrap(),
            )))
            .unwrap();

        Response { hyper_response }
    }
    pub fn ok_sse_stream<S: Stream<Item = sse::Event> + Sync + Send + 'static>(
        sse_stream: S
    ) -> Response {
        let hyper_body =
            hyper::Body::wrap_stream(sse_stream.map(|event| Ok::<_, Error>(event.serialize())));
        let hyper_response = hyper::Response::builder()
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(HttpBodyVariant::from(hyper_body))
            .unwrap();
        Response { hyper_response }
    }

    pub fn redirect_302(target: &str) -> Response {
        let hyper_response = hyper::Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, target)
            .body(HttpBodyVariant::from(hyper::Body::default()))
            .unwrap();
        Response { hyper_response }
    }

    pub fn error(status_code: StatusCode) -> Response {
        let hyper_response = hyper::Response::builder()
            .status(status_code)
            .body(HttpBodyVariant::from(hyper::Body::default()))
            .unwrap();

        Response { hyper_response }
    }
    pub fn error_400_from_error<T: Into<Error>>(error: T) -> Response {
        let hyper_response = hyper::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(HttpBodyVariant::from(hyper::Body::from(
                error.into().to_string(),
            )))
            .unwrap();
        Response { hyper_response }
    }
    pub fn error_404() -> Response {
        Self::error(StatusCode::NOT_FOUND)
    }
    pub fn error_500() -> Response {
        Self::error(StatusCode::NOT_FOUND)
    }
}

pub trait Handler {
    fn handle(
        &self,
        request: Request,
    ) -> BoxFuture<'static, Response>;
}
