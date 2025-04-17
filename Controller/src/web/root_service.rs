use super::{
    Handler, Request, Response,
    uri_cursor::{Handler as UriCursorHandler, UriCursor},
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
impl Handler for RootService<'_> {
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
                .respond(request.method(), request.uri().path(), request.headers());
        async { gui_response }.boxed()
    }
}

#[cfg(feature = "ci-packed-gui")]
mod gui_responder {
    use super::super::Response;
    use bytes::Bytes;
    use http::{HeaderMap, Method, Response as HttpResponse};
    use http_body_util::{BodyExt, combinators::BoxBody};
    use include_bytes_aligned::include_bytes_aligned;
    use std::env;
    use web_static_pack::{
        common::pack::PackArchived,
        loader::load,
        responder::{Responder, ResponderRespondError, Response as ResponderResponse},
    };

    static GUI_PACK_ARCHIVED: &[u8] = include_bytes_aligned!(16, env!("CI_WEB_STATIC_PACK_GUI"));

    #[derive(Debug)] // GuiResponderInner does not implement Debug
    pub struct GuiResponder {
        inner: Responder<'static, PackArchived>,
    }
    impl GuiResponder {
        pub fn new() -> Self {
            let pack_archived = unsafe { load(GUI_PACK_ARCHIVED) }.unwrap();
            let inner = Responder::new(pack_archived);
            Self { inner }
        }

        pub fn respond(
            &self,
            method: &Method,
            path: &str,
            headers: &HeaderMap,
        ) -> Response {
            let path = match path {
                "/" => "/index.html",
                path => path,
            };

            match self.inner.respond(method, path, headers) {
                Ok(response) => Self::convert_response(response),
                Err(error) => match error {
                    ResponderRespondError::PackPathNotFound => Response::redirect_302("/"),
                    error => Self::convert_response(error.into_response()),
                },
            }
        }

        fn convert_response(responder_response: ResponderResponse<'static>) -> Response {
            let (parts, body) = responder_response.into_parts();
            // impl Body<&[u8]> -> impl Body<Bytes>
            let body = body.map_frame(|frame| frame.map_data(Bytes::from_static));
            // impl Body<Bytes> -> BoxBody<Bytes>
            let body = BoxBody::new(body);

            // make HttpResponse<BoxBody<Bytes>, _>
            let http_response = HttpResponse::from_parts(parts, body);

            // make Response
            let response = Response::from_http_response(http_response);

            response
        }
    }
}

#[cfg(not(feature = "ci-packed-gui"))]
mod gui_responder {
    use super::super::Response;
    use http::{HeaderMap, Method};

    #[derive(Debug)]
    pub struct GuiResponder {}
    impl GuiResponder {
        pub fn new() -> Self {
            Self {}
        }

        pub fn respond(
            &self,
            _method: &Method,
            _path: &str,
            _headers: &HeaderMap,
        ) -> Response {
            Response::error_404()
        }
    }
}
