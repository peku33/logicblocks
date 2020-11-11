use crate::util::tokio_cancelable::ScopedSpawn;

use super::{Handler, Request, Response};
use futures::FutureExt;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request as HyperRequest, Response as HyperResponse, Server as HyperServer,
};
use owning_ref::OwningHandle;
use std::{convert::Infallible, net::SocketAddr};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

pub struct Server<'h> {
    runtime_context: OwningHandle<Box<Runtime>, Box<ScopedSpawn<'static, 'h, !>>>,
}
impl<'h> Server<'h> {
    pub fn new(
        bind: SocketAddr,
        handler: &'h (dyn Handler + Sync),
    ) -> Self {
        let runtime = RuntimeBuilder::new()
            .enable_all()
            .threaded_scheduler()
            .thread_name("Server.web")
            .build()
            .unwrap();

        let runtime_context = OwningHandle::new_with_fn(Box::new(runtime), |runtime_ptr| {
            let runtime = unsafe { &*runtime_ptr };

            let handler_unsafe =
                unsafe { std::mem::transmute::<_, &'static (dyn Handler + Sync)>(handler) };

            let scoped_spawn = ScopedSpawn::new(runtime, Self::serve(bind, handler_unsafe).boxed());

            Box::new(scoped_spawn)
        });

        Self { runtime_context }
    }
    pub async fn finalize(mut self) {
        // Cancel the server
        self.runtime_context.abort().await.unwrap_none();

        // Drop the runtime, so its not longer executed
        self.runtime_context.into_owner().shutdown_background();
    }

    async fn respond(
        handler: &'static (dyn Handler + Sync),
        remote_address: SocketAddr,
        hyper_request: HyperRequest<Body>,
    ) -> HyperResponse<Body> {
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

    async fn serve(
        bind: SocketAddr,
        handler: &'static (dyn Handler + Sync),
    ) -> ! {
        let make_service = make_service_fn(|connection: &AddrStream| {
            let remote_address = connection.remote_addr();
            async move {
                Ok::<_, Infallible>(service_fn(
                    move |hyper_request: HyperRequest<Body>| async move {
                        let hyper_response =
                            Self::respond(handler, remote_address, hyper_request).await;
                        Ok::<_, Infallible>(hyper_response)
                    },
                ))
            }
        });

        let server = HyperServer::bind(&bind).serve(make_service);
        match server.await {
            Ok(()) => panic!("server yielded without error"),
            Err(error) => panic!("server yielded with error: {:?}", error),
        }
    }
}
