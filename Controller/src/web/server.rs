use super::{Handler, Request, Response};
use crate::util::scoped_async::{ExitFlag, Exited, Runnable, ScopedRuntime};
use async_trait::async_trait;
use futures::{future::FutureExt, pin_mut, select};
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request as HyperRequest, Response as HyperResponse, Server as HyperServer,
};
use owning_ref::OwningHandle;
use std::{convert::Infallible, mem::transmute, net::SocketAddr};

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
            Ok(()) => panic!("server exited with no error"),
            Err(error) => panic!("server exited with error: {:?}", error),
        }
    }
}
#[async_trait]
impl<'h> Runnable for Server<'h> {
    async fn run(
        &self,
        mut exit_flag: ExitFlag,
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

pub struct ServerRunner<'h> {
    context: OwningHandle<Box<Server<'h>>, Box<ScopedRuntime<&'h Server<'h>>>>,
}
impl<'h> ServerRunner<'h> {
    pub fn new(
        bind: SocketAddr,
        handler: &'h (dyn Handler + Sync),
    ) -> Self {
        let server = Server::new(bind, handler);
        let context = OwningHandle::new_with_fn(Box::new(server), |server_ptr| {
            let server = unsafe { &*server_ptr };
            let scoped_runtime = ScopedRuntime::new(server, "Server.web".to_string());
            scoped_runtime.spawn_runnable_detached(|server| *server);
            Box::new(scoped_runtime)
        });
        Self { context }
    }
}
