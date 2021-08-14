use super::{Handler, Request, Response};
use crate::util::{
    async_flag,
    runtime::{Exited, FinalizeGuard, Runnable, Runtime, RuntimeScopeRunnable},
};
use anyhow::Context;
use async_trait::async_trait;
use futures::{future::FutureExt, pin_mut, select};
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request as HyperRequest, Response as HyperResponse, Server as HyperServer,
};
use ouroboros::self_referencing;
use std::{
    convert::Infallible,
    mem::{transmute, ManuallyDrop},
    net::SocketAddr,
};

pub struct Server<'h> {
    bind: SocketAddr,
    handler: &'h (dyn Handler + Sync),
}
impl<'h> Server<'h> {
    pub fn new(
        bind: SocketAddr,
        handler: &'h (dyn Handler + Sync),
    ) -> Self {
        Self { bind, handler }
    }

    async fn respond(
        &self,
        remote_address: SocketAddr,
        hyper_request: HyperRequest<Body>,
    ) -> HyperResponse<Body> {
        let (parts, body) = hyper_request.into_parts();
        let body = match hyper::body::to_bytes(body).await.context("to_bytes") {
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
            Ok(()) => panic!("server exited with no error"),
            Err(error) => panic!("server exited with error: {:?}", error),
        }
    }
}
#[async_trait]
impl<'h> Runnable for Server<'h> {
    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let run_future = unsafe { self.run() };
        pin_mut!(run_future);
        let mut run_future = run_future.fuse();

        select! {
            _ = run_future => panic!("run_future yielded"),
            () = exit_flag => Exited,
        }
    }
}

#[self_referencing]
struct ServerRunnerInner<'h> {
    server: Server<'h>,
    runtime: Runtime,

    #[borrows(server, runtime)]
    #[not_covariant]
    server_runtime_scope_runnable: ManuallyDrop<RuntimeScopeRunnable<'this, 'this, Server<'h>>>,
}
pub struct ServerRunner<'h> {
    inner: ServerRunnerInner<'h>,
    finalize_guard: FinalizeGuard,
}
impl<'h> ServerRunner<'h> {
    pub fn new(
        bind: SocketAddr,
        handler: &'h (dyn Handler + Sync),
    ) -> Self {
        let server = Server::new(bind, handler);

        let runtime = Runtime::new("web", 2, 2);

        let inner = ServerRunnerInnerBuilder {
            server,
            runtime,

            server_runtime_scope_runnable_builder: move |server, runtime| {
                let server_runtime_scope_runnable = RuntimeScopeRunnable::new(runtime, server);
                let server_runtime_scope_runnable =
                    ManuallyDrop::new(server_runtime_scope_runnable);
                server_runtime_scope_runnable
            },
        }
        .build();

        let finalize_guard = FinalizeGuard::new();

        Self {
            inner,
            finalize_guard,
        }
    }

    pub async fn finalize(mut self) {
        let server_runtime_scope_runnable =
            self.inner
                .with_server_runtime_scope_runnable_mut(|server_runtime_scope_runnable| {
                    let server_runtime_scope_runnable = unsafe {
                        transmute::<
                            &mut ManuallyDrop<RuntimeScopeRunnable<'_, '_, Server<'_>>>,
                            &mut ManuallyDrop<
                                RuntimeScopeRunnable<'static, 'static, Server<'static>>,
                            >,
                        >(server_runtime_scope_runnable)
                    };
                    let server_runtime_scope_runnable =
                        unsafe { ManuallyDrop::take(server_runtime_scope_runnable) };
                    server_runtime_scope_runnable
                });
        server_runtime_scope_runnable.finalize().await;

        self.finalize_guard.finalized();

        let inner_heads = self.inner.into_heads();
        drop(inner_heads);
    }
}
