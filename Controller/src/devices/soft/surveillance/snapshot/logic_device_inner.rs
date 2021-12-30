use crate::{web, web::uri_cursor};
use anyhow::{Context, Error};
use bytes::Bytes;
use futures::{
    future::{BoxFuture, Future, FutureExt},
    join,
};
use image::{imageops::FilterType, DynamicImage, ImageOutputFormat};
use parking_lot::RwLock;
use std::time::Duration;

#[derive(Debug)]
struct ManagerSize {
    max_width: Option<usize>,
    jpeg_quality: u8,
    jpeg_bytes: RwLock<Option<Bytes>>,
}
impl ManagerSize {
    pub fn new(
        max_width: Option<usize>,
        jpeg_quality: u8,
    ) -> Self {
        Self {
            max_width,
            jpeg_quality,
            jpeg_bytes: RwLock::new(None),
        }
    }

    pub async fn image_set(
        &self,
        image: DynamicImage,
    ) -> Result<(), Error> {
        let image = if let Some(max_width) = self.max_width {
            tokio::task::spawn_blocking(move || {
                image.resize(max_width as u32, u32::MAX, FilterType::Gaussian)
            })
            .await
            .context("spawn_blocking")?
        } else {
            image
        };

        let jpeg_quality = self.jpeg_quality;
        let jpeg_bytes = tokio::task::spawn_blocking(move || -> Result<Bytes, Error> {
            let mut jpeg_bytes = Vec::<u8>::new();
            image
                .write_to(&mut jpeg_bytes, ImageOutputFormat::Jpeg(jpeg_quality))
                .context("write_to")?;
            Ok(Bytes::from(jpeg_bytes))
        })
        .await
        .context("spawn_blocking")??;

        *self.jpeg_bytes.write() = Some(jpeg_bytes);

        Ok(())
    }
    pub fn image_unset(&self) {
        *self.jpeg_bytes.write() = None;
    }
}
impl uri_cursor::Handler for ManagerSize {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Terminal => match *request.method() {
                http::Method::GET => {
                    let jpeg_bytes = self.jpeg_bytes.read().clone();

                    async move {
                        match jpeg_bytes {
                            Some(jpeg_bytes) => {
                                web::Response::ok_content_type_body("image/jpeg", jpeg_bytes)
                            }
                            None => web::Response::error_404(),
                        }
                    }
                    .boxed()
                }
                _ => async move { web::Response::error_405() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}

#[derive(Debug)]
pub struct Manager {
    size_full: ManagerSize,
    size_320: ManagerSize,
}
impl Manager {
    pub fn new() -> Self {
        Self {
            size_full: ManagerSize::new(None, 95),
            size_320: ManagerSize::new(Some(320), 80),
        }
    }

    pub async fn image_set(
        &self,
        image: &DynamicImage,
    ) -> Result<(), Error> {
        let (result_full, result_320) = join!(
            self.size_full.image_set(image.clone()),
            self.size_320.image_set(image.clone()),
        );
        result_full.context("size_full image_set")?;
        result_320.context("size_320 image_set")?;

        Ok(())
    }
    pub fn image_unset(&self) {
        self.size_full.image_unset();
        self.size_320.image_unset();
    }
}
impl uri_cursor::Handler for Manager {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next("full", uri_cursor) => {
                self.size_full.handle(request, uri_cursor)
            }
            uri_cursor::UriCursor::Next("320", uri_cursor) => {
                self.size_320.handle(request, uri_cursor)
            }
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}

pub struct Runner<'s, G, F, U>
where
    G: Fn() -> F,
    F: Future<Output = Result<DynamicImage, Error>>,
    U: Fn(),
{
    manager: &'s Manager,
    getter: G,
    updated: U,
    interval: Duration,
}
impl<'s, G, F, U> Runner<'s, G, F, U>
where
    G: Fn() -> F,
    F: Future<Output = Result<DynamicImage, Error>>,
    U: Fn(),
{
    pub fn new(
        manager: &'s Manager,
        getter: G,
        updated: U,
        interval: Duration,
    ) -> Self {
        Self {
            manager,
            getter,
            updated,
            interval,
        }
    }

    pub async fn run_once(&self) -> Result<!, Error> {
        loop {
            let image = (self.getter)().await.context("getter")?;
            self.manager.image_set(&image).await.context("image_set")?;

            (self.updated)();

            tokio::time::sleep(self.interval).await;
        }
    }
}
