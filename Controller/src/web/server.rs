use super::{Handler, Request, Response};
use crate::{
    modules::module_path::ModulePath,
    util::{
        async_flag,
        drop_guard::DropGuard,
        runnable::{Exited, Runnable},
        runtime::{Runtime, RuntimeScopeRunnable},
    },
};
use anyhow::Context;
use async_trait::async_trait;
use futures::{future::FutureExt, pin_mut, select};
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request as HyperRequest, Response as HyperResponse, Server as HyperServer,
};
use once_cell::sync::Lazy;
use ouroboros::self_referencing;
use std::{
    convert::Infallible,
    mem::{transmute, ManuallyDrop},
    net::SocketAddr,
};

// #[derive(Debug)] // Debug not possible
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
    // To use it, you must ensure that all spawned futures ended (possibly by
    // waiting for runtime closing)
    async unsafe fn run(&self) -> ! {
        let self_static = transmute::<&'_ Server<'_>, &'static Server<'static>>(self);
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
        let runner = unsafe { self.run() };
        pin_mut!(runner);
        let mut runner = runner.fuse();

        select! {
            _ = runner => panic!("runner yielded"),
            () = exit_flag => Exited,
        }
    }
}

#[self_referencing]
// #[derive(Debug)] // Debug not possible
struct RunnerInner<'r, 'h: 'r> {
    server: Server<'h>,

    #[borrows(server)]
    #[not_covariant]
    runtime_scope_runnable: ManuallyDrop<RuntimeScopeRunnable<'r, 'this, Server<'h>>>,
}

pub struct Runner<'r, 'h> {
    inner: RunnerInner<'r, 'h>,

    drop_guard: DropGuard,
}
impl<'r, 'h> Runner<'r, 'h> {
    pub fn new(
        runtime: &'r Runtime,
        bind: SocketAddr,
        handler: &'h (dyn Handler + Sync),
    ) -> Self {
        let server = Server::new(bind, handler);

        let inner = RunnerInnerBuilder {
            server,
            runtime_scope_runnable_builder: |server| {
                let runtime_scope_runnable = RuntimeScopeRunnable::new(runtime, server);
                let runtime_scope_runnable = ManuallyDrop::new(runtime_scope_runnable);
                runtime_scope_runnable
            },
        }
        .build();

        let drop_guard = DropGuard::new();

        Self { inner, drop_guard }
    }

    pub async fn finalize(mut self) {
        let runtime_scope_runnable =
            self.inner
                .with_runtime_scope_runnable_mut(|runtime_scope_runnable| unsafe {
                    ManuallyDrop::take(runtime_scope_runnable)
                });
        runtime_scope_runnable.finalize().await;

        self.drop_guard.set();
    }
}

#[self_referencing]
// #[derive(Debug)] // Debug not possible
struct RunnerOwnedInner<'h> {
    runtime: Runtime,

    #[borrows(runtime)]
    #[not_covariant]
    runner: ManuallyDrop<Runner<'this, 'h>>,
}

// #[derive(Debug)] // Debug not possible
pub struct RunnerOwned<'h> {
    inner: RunnerOwnedInner<'h>,

    drop_guard: DropGuard,
}
impl<'h> RunnerOwned<'h> {
    fn module_path() -> &'static ModulePath {
        static MODULE_PATH: Lazy<ModulePath> = Lazy::new(|| ModulePath::new(&["web", "server"]));
        &MODULE_PATH
    }

    pub fn new(
        bind: SocketAddr,
        handler: &'h (dyn Handler + Sync),
    ) -> Self {
        let runtime = Runtime::new(Self::module_path(), 2, 2);

        let inner = RunnerOwnedInnerBuilder {
            runtime,

            runner_builder: |runtime| {
                let runner = Runner::new(runtime, bind, handler);
                let runner = ManuallyDrop::new(runner);
                runner
            },
        }
        .build();

        let drop_guard = DropGuard::new();

        Self { inner, drop_guard }
    }

    pub async fn finalize(mut self) {
        let runner = self
            .inner
            .with_runner_mut(|runner| unsafe { ManuallyDrop::take(runner) });
        runner.finalize().await;

        self.drop_guard.set();

        let inner_heads = self.inner.into_heads();
        drop(inner_heads);
    }
}
