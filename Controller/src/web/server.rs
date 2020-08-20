use super::{hyper_body_variant::HttpBodyVariant, Handler, Request, Response};
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request as HyperRequest, Response as HyperResponse, Server,
};
use std::{convert::Infallible, net::SocketAddr};

async fn respond(
    handler: &'static (dyn Handler + Sync),
    remote_address: SocketAddr,
    hyper_request: HyperRequest<Body>,
) -> HyperResponse<HttpBodyVariant> {
    let (parts, body) = hyper_request.into_parts();
    let body = match hyper::body::to_bytes(body).await {
        Ok(body) => body,
        Err(error) => return Response::error_400_from_error(error).into_hyper_response(),
    };

    let request = Request::new(remote_address, parts, body);
    let log_method = request.method().clone();
    let log_uri = request.uri().clone();

    let response = handler.handle(request).await;
    let log_status_code = response.status_code();

    log::trace!(
        "{:?} {} {} {}",
        remote_address,
        log_method,
        log_uri,
        log_status_code,
    );

    response.into_hyper_response()
}

pub async fn serve(
    bind: SocketAddr,
    handler: &(dyn Handler + Sync),
) -> ! {
    let handler_unsafe =
        unsafe { std::mem::transmute::<_, &'static (dyn Handler + Sync)>(handler) };

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
        Ok(()) => panic!("server yielded without error"),
        Err(error) => panic!("server yielded with error: {:?}", error),
    }
}
