use super::{
    uri_cursor::{Handler as UriCursorHandler, UriCursor},
    Handler, Request, Response,
};
use futures::{
    future::{ready, BoxFuture},
    FutureExt,
};

#[cfg(feature = "ci-packed-gui")]
use owning_ref::OwningHandle;
#[cfg(feature = "ci-packed-gui")]
use web_static_pack::hyper_loader::Responder;
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
}
impl<'a> Handler for RootService<'a> {
    fn handle(
        &self,
        request: Request,
    ) -> BoxFuture<'static, Response> {
        // Extract request path
        let path = request.uri().path();

        // Redirect / to /index.html
        if path == "/" {
            return ready(Response::redirect_302("/index.html")).boxed();
        }

        // Serve API if url starts with /api
        let api_prefix = "/api/";
        if path.starts_with(api_prefix) {
            let uri_cursor_left = path[api_prefix.len()..].to_owned();
            let uri_cursor = UriCursor::new(uri_cursor_left);
            return self.api_handler.handle(request, uri_cursor);
        }

        // Serve GUI
        #[cfg(feature = "ci-packed-gui")]
        return ready(
            match self.gui_responder.parts_respond_or_error(
                request.method(),
                request.uri(),
                request.headers(),
            ) {
                Ok(response) => Response::wrap_web_static_pack_response(response),
                Err(error) => Response::error(error.as_http_status_code()),
            },
        )
        .boxed();

        #[cfg(not(feature = "ci-packed-gui"))]
        return ready(Response::error_404()).boxed();
    }
}
