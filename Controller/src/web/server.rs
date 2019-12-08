use super::{HandlerThreaded, Request};
use failure::{err_msg, Error};
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
                        let body = body.map_ok(|chunk| chunk.into_bytes()).try_concat().await?; // FIXME: Possible future exit
                        let handler_request = Request::new(remote_address, http_fields, body);

                        let handler_response =
                            handler_static.handle(Box::new(handler_request)).await;
                        let mut hyper_response = handler_response.into_hyper_response();

                        if let Some(cors) = cors.as_ref() {
                            hyper_response.headers_mut().append(
                                http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                                cors.parse().unwrap(),
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
