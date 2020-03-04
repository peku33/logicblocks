use super::hyper_body_variant::HttpBodyVariant;
use super::{Handler, Request, Response};
use failure::{err_msg, Error};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request as HyperRequest, Response as HyperResponse, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

async fn respond(
    handler: &'static (dyn Handler + Sync + Send),
    remote_address: SocketAddr,
    hyper_request: HyperRequest<Body>,
) -> HyperResponse<HttpBodyVariant> {
    let (parts, body) = hyper_request.into_parts();
    let body = match hyper::body::to_bytes(body).await {
        Ok(body) => body,
        Err(error) => return Response::error_400_from_error(error).into_hyper_response(),
    };

    let request = Request::new(remote_address, parts, body);
    let response = handler.handle(request).await;
    response.into_hyper_response()
}

pub async fn serve(
    bind: SocketAddr,
    handler: &(dyn Handler + Sync + Send),
) -> Error {
    let handler_unsafe =
        unsafe { std::mem::transmute::<_, &'static (dyn Handler + Sync + Send)>(handler) };

    let make_service = make_service_fn(|connection: &AddrStream| {
        let remote_address = connection.remote_addr();
        async move {
            Ok::<_, Infallible>(service_fn(
                move |hyper_request: HyperRequest<Body>| async move {
                    let hyper_response =
                        respond(handler_unsafe, remote_address, hyper_request).await;
                    Ok::<_, Infallible>(hyper_response)
                },
            ))
        }
    });

    let server = Server::bind(&bind).serve(make_service);
    match server.await {
        Ok(()) => err_msg("server returned with no error"),
        Err(error) => error.into(),
    }
}
