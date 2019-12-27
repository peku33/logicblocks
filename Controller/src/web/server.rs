use super::{HandlerThreaded, Request};
use bytes::BytesMut;
use failure::{err_msg, Error};
use futures::future::TryFutureExt;
use futures::stream::TryStreamExt;
use std::net::SocketAddr;

pub async fn run_server(
    bind: SocketAddr,
    handler: &(dyn HandlerThreaded),
    cors: Option<&'static str>,
) -> Error {
    // FIXME: https://github.com/hyperium/hyper/issues/1669
    let handler_static = unsafe {
        std::mem::transmute::<&(dyn HandlerThreaded), &'static (dyn HandlerThreaded)>(handler)
    };

    let make_service =
        hyper::service::make_service_fn(|socket: &hyper::server::conn::AddrStream| {
            let remote_address = socket.remote_addr();
            return async move {
                return Ok::<_, hyper::Error>(hyper::service::service_fn(
                    async move |hyper_request: hyper::Request<hyper::Body>| {
                        let (http_fields, body) = hyper_request.into_parts();

                        let mut hyper_response = match http_fields.method {
                            http::Method::OPTIONS => hyper::Response::builder()
                                .body(hyper::Body::default())
                                .unwrap(),
                            _ => match body
                                .try_fold(BytesMut::new(), |mut buffer, chunk| {
                                    buffer.extend_from_slice(&chunk);
                                    async move { Ok(buffer) }
                                })
                                .map_ok(|buffer| buffer.freeze())
                                .await
                            {
                                Ok(body) => {
                                    let handler_request =
                                        Request::new(remote_address, http_fields, body);
                                    let handler_response =
                                        handler_static.handle(Box::new(handler_request)).await;
                                    handler_response.into_hyper_response()
                                }
                                Err(_) => hyper::Response::builder()
                                    .status(http::StatusCode::BAD_REQUEST)
                                    .body(hyper::Body::default())
                                    .unwrap(),
                            },
                        };

                        if let Some(cors) = cors.as_ref() {
                            let headers = hyper_response.headers_mut();
                            headers.append(
                                http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                                cors.parse().unwrap(),
                            );
                            headers.append(
                                http::header::ACCESS_CONTROL_ALLOW_HEADERS,
                                "Content-Type".parse().unwrap(),
                            );
                        }

                        return Ok::<_, hyper::Error>(hyper_response);
                    },
                ));
            };
        });

    let server = hyper::Server::bind(&bind).serve(make_service);

    return match server.await {
        Ok(()) => err_msg("Server exited with ()"),
        Err(e) => e.into(),
    };
}
