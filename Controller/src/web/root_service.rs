use super::{
    uri_cursor::{Handler as UriCursorHandler, UriCursor},
    Handler, Request, Response,
};
use futures::future::{BoxFuture, FutureExt};
use http::{HeaderMap, Method, Uri};

#[cfg(feature = "ci-packed-gui")]
use owning_ref::OwningHandle;
#[cfg(feature = "ci-packed-gui")]
use web_static_pack::hyper_loader::{Responder, ResponderError};
#[cfg(feature = "ci-packed-gui")]
use web_static_pack::loader::Loader;

pub struct RootService<'a> {
    api_handler: &'a (dyn UriCursorHandler + Sync),

    #[cfg(feature = "ci-packed-gui")]
    gui_responder: OwningHandle<Box<Loader>, Box<Responder<'static>>>,
}
impl<'a> RootService<'a> {
    pub fn new(api_handler: &'a (dyn UriCursorHandler + Sync)) -> Self {
        #[cfg(feature = "ci-packed-gui")]
        let gui_responder = OwningHandle::new_with_fn(
            Box::new(
                Loader::new(std::include_bytes!(std::env!("CI_WEB_STATIC_PACK_GUI"))).unwrap(),
            ),
            |loader_ptr| unsafe { Box::new(Responder::new(&*loader_ptr)) },
        );

        Self {
            api_handler,
            #[cfg(feature = "ci-packed-gui")]
            gui_responder,
        }
    }

    #[cfg(feature = "ci-packed-gui")]
    fn gui_responder_respond(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
    ) -> Response {
        // If path is /, use index.html
        if uri.path() == "/" {
            match self.gui_responder.parts_respond_or_error(
                method,
                &Uri::from_static("/index.html"),
                headers,
            ) {
                Ok(response) => return Response::from_hyper_response(response),
                Err(error) => return Response::error(error.as_http_status_code()),
            };
        }

        // Try actual file
        match self
            .gui_responder
            .parts_respond_or_error(method, uri, headers)
        {
            Ok(response) => return Response::from_hyper_response(response),
            Err(error) => match error {
                ResponderError::LoaderPathNotFound => (),
                _ => return Response::error(error.as_http_status_code()),
            },
        };

        // Fallback to /
        Response::redirect_302("/")
    }

    #[cfg(not(feature = "ci-packed-gui"))]
    fn gui_responder_respond(
        &self,
        _method: &Method,
        _uri: &Uri,
        _headers: &HeaderMap,
    ) -> Response {
        Response::error_404()
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
            self.gui_responder_respond(request.method(), request.uri(), request.headers());
        async move { gui_response }.boxed()
    }
}
