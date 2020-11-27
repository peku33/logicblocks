use super::{Handler, Request, Response};
use crate::util::scoped_async::{Runnable, ScopedRunnerSync};
use async_trait::async_trait;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request as HyperRequest, Response as HyperResponse, Server as HyperServer,
};
use owning_ref::OwningHandle;
use std::{convert::Infallible, mem::transmute, net::SocketAddr};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

pub struct Server<'h> {
    bind: SocketAddr,
    handler: &'h (dyn Handler + Sync),
}
impl<'h> Server<'h> {
    async fn respond(
        &self,
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

        let response = self.handler.handle(request).await;
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

    // Unsafe because of transmuting.
    // To use it, you must ensure that all spawned futures ended (possibly by waiting for runtime closing)
    async unsafe fn run(&self) -> ! {
        let self_static = transmute::<_, &'static Server<'static>>(self);
        let make_service = make_service_fn(|connection: &AddrStream| {
            let remote_address = connection.remote_addr();
            async move {
                Ok::<_, Infallible>(service_fn(
                    move |hyper_request: HyperRequest<Body>| async move {
                        let hyper_response =
                            self_static.respond(remote_address, hyper_request).await;
                        Ok::<_, Infallible>(hyper_response)
                    },
                ))
            }
        });

        let server = HyperServer::bind(&self.bind).serve(make_service);
        match server.await {
            Ok(()) => panic!("server yielded without error"),
            Err(error) => panic!("server yielded with error: {:?}", error),
        }
    }
}
#[async_trait]
impl<'h> Runnable for Server<'h> {
    async fn run(&self) -> ! {
        unsafe { self.run().await }
    }

    async fn finalize(&self) {}
}

struct ServerRunnerContextOwner<'h> {
    runtime: Runtime,
    server: Server<'h>,
}
struct ServerRunnerContextHandle<'r, 'u> {
    server_scoped_runner: ScopedRunnerSync<'r, 'u>,
}

pub struct ServerRunner<'h> {
    context:
        OwningHandle<Box<ServerRunnerContextOwner<'h>>, Box<ServerRunnerContextHandle<'h, 'h>>>,
}
impl<'h> ServerRunner<'h> {
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

        let server = Server { bind, handler };

        let server_runner_context_owner = ServerRunnerContextOwner { runtime, server };

        let context = OwningHandle::new_with_fn(
            Box::new(server_runner_context_owner),
            |server_runner_context_owner_ptr| {
                let server_runner_context_owner = unsafe { &*server_runner_context_owner_ptr };

                let server_runner_context_handle = ServerRunnerContextHandle {
                    server_scoped_runner: ScopedRunnerSync::new(
                        &server_runner_context_owner.runtime,
                        &server_runner_context_owner.server,
                    ),
                };
                Box::new(server_runner_context_handle)
            },
        );

        Self { context }
    }
}
