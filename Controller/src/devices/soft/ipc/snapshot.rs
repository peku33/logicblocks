use crate::util::Captures;
use crate::web::uri_cursor::{Handler, UriCursor};
use crate::web::{Request, Response};
use failure::Error;
use futures::future::{ready, BoxFuture, FutureExt, LocalBoxFuture};
use futures::stream::{unfold, Stream};
use std::cell::RefCell;
use std::time::Duration;

pub type Image = image::DynamicImage;
pub type ProviderResult = Result<Image, Error>;
pub type ProviderClosure<'p> = dyn Fn() -> LocalBoxFuture<'p, ProviderResult>;

static THUMBNAIL_SMALL_SIZE: (u32, u32) = (320, 240);

#[derive(Default)]
pub struct State {
    image_full_cache: Option<Image>,
    image_thumbnail_small_cache: Option<Image>,
}
impl State {
    pub fn image_full(&self) -> Option<&Image> {
        return self.image_full_cache.as_ref();
    }
    pub fn image_thumbnail_small(&mut self) -> Option<&Image> {
        if self.image_thumbnail_small_cache.is_none() && self.image_full_cache.is_some() {
            let image_thumbnail_small_cache = self
                .image_full_cache
                .as_ref()
                .unwrap()
                .thumbnail(THUMBNAIL_SMALL_SIZE.0, THUMBNAIL_SMALL_SIZE.1);
            self.image_thumbnail_small_cache
                .replace(image_thumbnail_small_cache);
        }
        return self.image_thumbnail_small_cache.as_ref();
    }

    pub fn set_image_full(
        &mut self,
        image: Image,
    ) -> () {
        self.image_full_cache.replace(image);
        self.image_thumbnail_small_cache = None;
    }

    pub fn clear(&mut self) {
        self.image_full_cache = None;
        self.image_thumbnail_small_cache = None;
    }
}

pub struct Driver<'p> {
    provider: Box<ProviderClosure<'p>>,
    interval: Duration,

    state: RefCell<State>,
}
impl<'p> Driver<'p> {
    pub fn new(
        provider: Box<ProviderClosure<'p>>,
        interval: Duration,
    ) -> Self {
        return Self {
            provider,
            interval,
            state: RefCell::default(),
        };
    }

    pub fn has_image(&self) -> bool {
        return self.state.borrow().image_full_cache.is_some();
    }
    pub fn reset(&self) -> () {
        self.state.borrow_mut().clear();
    }

    pub fn run<'s>(&'s self) -> impl Stream<Item = ()> + Captures<'p> + Captures<'s> {
        return unfold(true, async move |first| {
            if !first {
                tokio::time::delay_for(self.interval).await;
            }

            let snapshot_result = (self.provider)().await;
            match snapshot_result {
                Ok(snapshot) => {
                    self.state.borrow_mut().set_image_full(snapshot);
                }
                Err(e) => {
                    log::warn!("Error while obtaining snapshot: {:?}", e);
                    self.state.borrow_mut().clear();
                }
            }
            return Some(((), false));
        });
    }
}
impl Handler for Driver<'_> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        let image_quality = match (request.method(), uri_cursor.rest()) {
            (&http::Method::GET, "") => self
                .state
                .borrow()
                .image_full()
                .map(|image| (image.clone(), 95)),
            (&http::Method::GET, "small") => self
                .state
                .borrow_mut()
                .image_thumbnail_small()
                .map(|image| (image.clone(), 85)),
            _ => None,
        };

        let (image, quality) = match image_quality {
            Some((image, quality)) => (image, quality),
            None => return ready(Response::error(http::StatusCode::SERVICE_UNAVAILABLE)).boxed(),
        };

        return async move {
            let mut body = Vec::new();
            image
                .write_to(&mut body, image::ImageOutputFormat::Jpeg(quality))
                .unwrap();

            return Response::ok_content_type_body(body, "image/jpeg");
        }
        .boxed();
    }
}
