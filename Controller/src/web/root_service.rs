use super::{
    uri_cursor::{Handler as UriCursorHandler, UriCursor},
    Handler, Request, Response,
};
use futures::future::{BoxFuture, FutureExt};

// #[derive(Debug)] // Debug not possible
pub struct RootService<'a> {
    api_handler: &'a (dyn UriCursorHandler + Sync),
    gui_responder: gui_responder::GuiResponder,
}
impl<'a> RootService<'a> {
    pub fn new(api_handler: &'a (dyn UriCursorHandler + Sync)) -> Self {
        let gui_responder = gui_responder::GuiResponder::new();

        Self {
            api_handler,
            gui_responder,
        }
    }
}
impl<'a> Handler for RootService<'a> {
    fn handle(
        &self,
        request: Request,
    ) -> BoxFuture<'static, Response> {
        // Extract request path
        let path = request.uri().path().to_owned();

        // Serve API if url starts with /api
        if let Some(api_path) = path.strip_prefix("/api/") {
            let uri_cursor = UriCursor::new(api_path);
            return self.api_handler.handle(request, &uri_cursor);
        }

        // Serve GUI
        let gui_response =
            self.gui_responder
                .respond(request.method(), request.uri(), request.headers());
        async move { gui_response }.boxed()
    }
}

#[cfg(feature = "ci-packed-gui")]
mod gui_responder {
    use super::super::Response;
    use http::{HeaderMap, Method, Uri};
    use ouroboros::self_referencing;
    use std::{env, include_bytes};
    use web_static_pack::{
        hyper_loader::{Responder, ResponderError},
        loader::Loader,
    };

    #[self_referencing]
    // #[derive(Debug)] // Loader & Responder does not implement Debug
    struct GuiResponderInner {
        loader: Loader,

        #[borrows(loader)]
        #[covariant]
        responder: Responder<'this>,
    }

    // #[derive(Debug)] // GuiResponderInner does not implement Debug
    pub struct GuiResponder {
        inner: GuiResponderInner,
    }
    impl GuiResponder {
        pub fn new() -> Self {
            let inner = GuiResponderInnerBuilder {
                loader: Loader::new(include_bytes!(env!("CI_WEB_STATIC_PACK_GUI"))).unwrap(),
                responder_builder: move |loader| Responder::new(loader),
            }
            .build();

            Self { inner }
        }

        pub fn respond(
            &self,
            method: &Method,
            uri: &Uri,
            headers: &HeaderMap,
        ) -> Response {
            let responder = self.inner.borrow_responder();

            // If path is /, use index.html
            if uri.path() == "/" {
                match responder.parts_respond_or_error(
                    method,
                    &Uri::from_static("/index.html"),
                    headers,
                ) {
                    Ok(response) => return Response::from_hyper_response(response),
                    Err(error) => return Response::error(error.as_http_status_code()),
                };
            }

            // Try actual file
            match responder.parts_respond_or_error(method, uri, headers) {
                Ok(response) => return Response::from_hyper_response(response),
                Err(error) => match error {
                    ResponderError::LoaderPathNotFound => (),
                    _ => return Response::error(error.as_http_status_code()),
                },
            };

            // Fallback to /
            Response::redirect_302("/")
        }
    }
}

#[cfg(not(feature = "ci-packed-gui"))]
mod gui_responder {
    use super::super::Response;
    use http::{HeaderMap, Method, Uri};

    #[derive(Debug)]
    pub struct GuiResponder {}
    impl GuiResponder {
        pub fn new() -> Self {
            Self {}
        }

        pub fn respond(
            &self,
            _method: &Method,
            _uri: &Uri,
            _headers: &HeaderMap,
        ) -> Response {
            Response::error_404()
        }
    }
}
