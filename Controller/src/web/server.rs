use super::{Handler, Request, ResponseFull};
use crate::{
    modules::module_path::ModulePath,
    util::{
        async_flag,
        drop_guard::DropGuard,
        runnable::{Exited, Runnable},
        runtime::{Runtime, RuntimeScopeRunnable},
    },
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{
    future::{Either, FutureExt, select},
    pin_mut, select,
};
use http::{request::Request as HttpRequest, response::Response as HttpResponse};
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
use hyper::{body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::{conn::auto::Builder, graceful::GracefulShutdown},
};
use once_cell::sync::Lazy;
use ouroboros::self_referencing;
use std::{
    convert::Infallible,
    fmt,
    mem::{ManuallyDrop, transmute},
    net::SocketAddr,
    time::Duration,
};
use tokio::net::TcpListener;

#[derive(derive_more::Debug)]
pub struct Server<'h> {
    bind: SocketAddr,
    #[debug(skip)]
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
        http_request: HttpRequest<Incoming>,
        exit_flag_template: &async_flag::Receiver,
    ) -> HttpResponse<UnsyncBoxBody<Bytes, Infallible>> {
        let (parts, body) = http_request.into_parts();
        // TODO: we probably want to limit incoming body size here?
        let body = match body.collect().await.context("collect") {
            Ok(body) => body.to_bytes(),
            Err(error) => {
                return ResponseFull::error_400_from_error(error)
                    .into_http_response()
                    .map(|body| body.boxed_unsync());
            }
        };

        let request = Request::from_http_request(remote_address, parts, body);
        let log_method = request.method().clone();
        let log_uri = request.uri().clone();

        let response = self.handler.handle(request).await;
        let http_response = response.into_http_response(exit_flag_template);
        let log_status_code = http_response.status();

        log::debug!(
            "{self}: {remote_address:?} {log_method} {log_uri} {log_status_code}",
        );

        http_response
    }

    async fn run_once(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        let listener = TcpListener::bind(self.bind).await.context("bind")?;
        log::trace!("{self}: server listening");

        let server = Builder::new(TokioExecutor::new());
        let graceful = GracefulShutdown::new();

        // SAFETY: we guarantee that all connections are closed before leaving
        // this function, so '_ will outlive the hyper server
        let self_static = unsafe { transmute::<&'_ Server<'_>, &'static Server<'static>>(self) };
        let exit_flag_static = unsafe {
            transmute::<&'_ async_flag::Receiver, &'static async_flag::Receiver>(&exit_flag)
        };

        let mut server_exit_flag = exit_flag.clone();
        loop {
            let listener_accept = listener.accept();
            pin_mut!(listener_accept);

            match select(listener_accept, &mut server_exit_flag).await {
                Either::Left((connection, _)) => {
                    let (stream, remote_address) = match connection.context("connection") {
                        Ok(connection) => connection,
                        Err(error) => {
                            log::error!("{self}: connection error: {error:?}");
                            continue;
                        }
                    };

                    let io = TokioIo::new(stream);

                    let connection = server.serve_connection(
                        io,
                        service_fn(move |http_request| async move {
                            let response = self_static
                                .respond(remote_address, http_request, exit_flag_static)
                                .await;
                            Ok::<_, Infallible>(response)
                        }),
                    );

                    let connection_watch = graceful.watch(connection.into_owned());

                    tokio::spawn(async move {
                        match connection_watch.await {
                            Ok(()) => {}
                            Err(error) => {
                                // don't log hyper::Error(IncompleteMessage) which originates from
                                // ex. infinite sse streams
                                let mut log_ = true;

                                if let Some(error) = error.downcast_ref::<hyper::Error>()
                                    && (error.is_incomplete_message() || error.is_canceled()) {
                                        log_ = false;
                                    }

                                if log_ {
                                    log::error!("{self_static}: connection error: {error:?}");
                                }
                            }
                        };
                    });
                }
                Either::Right(((), _)) => {
                    log::trace!("{self}: received exit signal");
                    break;
                }
            }
        }

        // stop accepting new connections
        drop(listener);

        // shutdown all connections
        log::trace!("{self}: waiting for all remaining connections to shutdown");
        graceful.shutdown().await;
        log::trace!("{self}: all remaining connections closed");

        Ok(Exited)
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        const ERROR_RESTART_DELAY: Duration = Duration::from_secs(5);

        loop {
            let error = match self.run_once(exit_flag.clone()).await.context("run_once") {
                Ok(Exited) => break,
                Err(error) => error,
            };
            log::error!("{self}: {error:?}");

            select! {
                () = tokio::time::sleep(ERROR_RESTART_DELAY).fuse() => {},
                () = exit_flag => break,
            }
        }

        Exited
    }
}
#[async_trait]
impl Runnable for Server<'_> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
impl fmt::Display for Server<'_> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Server ({:?})", self.bind)
    }
}

#[self_referencing]
#[derive(Debug)]
struct RunnerInner<'r, 'h: 'r> {
    server: Server<'h>,

    #[borrows(server)]
    #[not_covariant]
    runtime_scope_runnable: ManuallyDrop<RuntimeScopeRunnable<'r, 'this, Server<'h>>>,
}

#[derive(Debug)]
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
#[derive(Debug)]
struct RunnerOwnedInner<'h> {
    runtime: Runtime,

    #[borrows(runtime)]
    #[not_covariant]
    runner: ManuallyDrop<Runner<'this, 'h>>,
}

#[derive(Debug)]
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
