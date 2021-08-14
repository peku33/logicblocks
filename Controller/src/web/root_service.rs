use super::{
    uri_cursor::{Handler as UriCursorHandler, UriCursor},
    Handler, Request, Response,
};
use futures::future::{BoxFuture, FutureExt};
use http::{HeaderMap, Method, Uri};

#[cfg(feature = "ci-packed-gui")]
use ouroboros::self_referencing;
#[cfg(feature = "ci-packed-gui")]
use web_static_pack::hyper_loader::{Responder, ResponderError};
#[cfg(feature = "ci-packed-gui")]
use web_static_pack::loader::Loader;

#[cfg(feature = "ci-packed-gui")]
#[self_referencing]
struct GuiResponderInner {
    loader: Loader,

    #[borrows(loader)]
    #[not_covariant]
    responder: Responder<'this>,
}

pub struct RootService<'a> {
    api_handler: &'a (dyn UriCursorHandler + Sync),

    #[cfg(feature = "ci-packed-gui")]
    gui_responder: GuiResponderInner,
}
impl<'a> RootService<'a> {
    pub fn new(api_handler: &'a (dyn UriCursorHandler + Sync)) -> Self {
        #[cfg(feature = "ci-packed-gui")]
        let gui_responder = GuiResponderInnerBuilder {
            loader: Loader::new(std::include_bytes!(std::env!("CI_WEB_STATIC_PACK_GUI"))).unwrap(),
            responder_builder: |loader| Responder::new(loader),
        }
        .build();

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
        let responder: &Responder<'static> =
            self.gui_responder.with_responder(|responder| unsafe {
                #[allow(clippy::transmute_ptr_to_ptr)]
                std::mem::transmute::<&Responder<'_>, &Responder<'static>>(responder)
            });

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
