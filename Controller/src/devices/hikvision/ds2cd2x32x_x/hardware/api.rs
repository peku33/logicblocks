use super::boundary_stream;
use anyhow::{anyhow, bail, ensure, Context, Error};
use bytes::Bytes;
use futures::stream::{BoxStream, Stream, StreamExt};
use http::{
    uri::{self, Authority, PathAndQuery, Scheme},
    Method, Uri,
};
use image::DynamicImage;
use semver::{Version, VersionReq};
use std::{pin::Pin, task, time::Duration};
use xmltree::Element;

#[derive(Debug)]
pub struct PushResponse {
    pub reboot_required: bool,
    pub id: Option<usize>,
}

#[derive(Debug)]
pub struct PostResponse {
    pub reboot_required: bool,
    pub id: usize,
}
#[derive(Debug)]
pub struct PutResponse {
    pub reboot_required: bool,
}
#[derive(Debug)]
pub struct DeleteResponse {
    pub reboot_required: bool,
}

#[derive(Debug)]
pub struct BasicDeviceInfo {
    pub model: String,
    pub firmware_version: Version,
}

#[derive(Debug)]
pub enum VideoStream {
    Main,
    Sub,
}

#[derive(Debug)]
pub struct Api {
    host: Authority,
    admin_password: String,

    reqwest_client: reqwest::Client,
}
impl Api {
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

    pub fn new(
        host: Authority,
        admin_password: String,
    ) -> Self {
        let reqwest_client = reqwest::ClientBuilder::new().build().unwrap();

        Self {
            host,
            admin_password,

            reqwest_client,
        }
    }

    fn parse_xml(input: Bytes) -> Result<Element, Error> {
        let output = Element::parse(&input as &[u8]).context("parse")?;
        Ok(output)
    }
    fn serialize_xml(input: Element) -> Result<Bytes, Error> {
        let mut output = Vec::new();
        input.write(&mut output).context("write")?;
        let output: Bytes = output.into();
        Ok(output)
    }

    fn handle_push_response(response: Element) -> Result<PushResponse, Error> {
        ensure!(
            response.name == "ResponseStatus",
            "got invalid response root object: {}",
            response.name
        );

        let status_code: u8 = response
            .get_child("statusCode")
            .ok_or_else(|| anyhow!("missing statusCode"))?
            .get_text()
            .ok_or_else(|| anyhow!("missing statusCode text"))?
            .parse()
            .context("statusCode")?;

        let status_string = response
            .get_child("statusString")
            .ok_or_else(|| anyhow!("missing statusString"))?
            .get_text()
            .ok_or_else(|| anyhow!("missing statusString text"))?;

        let id = match response.get_child("id") {
            Some(id) => Some(
                id.get_text()
                    .ok_or_else(|| anyhow!("missing id text"))?
                    .parse()
                    .context("id parse")?,
            ),
            None => None,
        };

        let reboot_required = match status_code {
            1 => false,
            7 => true,
            status_code => {
                bail!(
                    "operation failed with status_code: {} ({})",
                    status_code,
                    status_string
                );
            }
        };
        Ok(PushResponse {
            reboot_required,
            id,
        })
    }

    fn url_build(
        &self,
        path_and_query: PathAndQuery,
    ) -> Uri {
        uri::Builder::new()
            .scheme(Scheme::HTTP)
            .authority(self.host.clone())
            .path_and_query(path_and_query)
            .build()
            .unwrap()
    }

    pub async fn request_bytes(
        &self,
        method: Method,
        path_and_query: PathAndQuery,
    ) -> Result<Bytes, Error> {
        let request = self
            .reqwest_client
            .request(method, &self.url_build(path_and_query).to_string())
            .timeout(Self::REQUEST_TIMEOUT)
            .basic_auth("admin", Some(&self.admin_password))
            .header(http::header::ACCEPT, "application/octet-stream");

        let response = request
            .send()
            .await
            .context("send")?
            .error_for_status()
            .context("error_for_status")?
            .bytes()
            .await
            .context("bytes")?;

        Ok(response)
    }
    pub async fn request_xml(
        &self,
        method: Method,
        path_and_query: PathAndQuery,
        input: Option<Element>,
    ) -> Result<Element, Error> {
        let mut request = self
            .reqwest_client
            .request(method, &self.url_build(path_and_query).to_string())
            .timeout(Self::REQUEST_TIMEOUT)
            .basic_auth("admin", Some(&self.admin_password))
            .header(http::header::ACCEPT, "application/xml");

        if let Some(input) = input {
            request = request
                .header(http::header::CONTENT_TYPE, "application/xml")
                .body(Self::serialize_xml(input).context("serialize_xml")?);
        }

        let response = request
            .send()
            .await
            .context("send")?
            .error_for_status()
            .context("error_for_status")?
            .bytes()
            .await
            .context("bytes")?;

        let output = Self::parse_xml(response).context("parse_xml")?;

        Ok(output)
    }
    pub async fn request_boundary_stream(
        &self,
        path_and_query: PathAndQuery,
    ) -> Result<BoundaryStreamExtractor, Error> {
        let request = self
            .reqwest_client
            .request(Method::GET, &self.url_build(path_and_query).to_string())
            .basic_auth("admin", Some(&self.admin_password))
            .header(http::header::ACCEPT, "multipart/mixed");

        let response = request
            .send()
            .await
            .context("send")?
            .error_for_status()
            .context("error_for_status")?;

        let content_type = response
            .headers()
            .get(http::header::CONTENT_TYPE)
            .ok_or_else(|| anyhow!("missing content type"))?;

        ensure!(content_type == "multipart/mixed; boundary=boundary");

        let data_stream = response.bytes_stream().boxed();

        let boundary_stream_extractor = BoundaryStreamExtractor::new(data_stream);

        Ok(boundary_stream_extractor)
    }

    pub async fn get_xml(
        &self,
        path_and_query: PathAndQuery,
    ) -> Result<Element, Error> {
        let response = self
            .request_xml(Method::GET, path_and_query, None)
            .await
            .context("request_xml")?;

        Ok(response)
    }
    pub async fn post_xml(
        &self,
        path_and_query: PathAndQuery,
        input: Option<Element>,
    ) -> Result<PostResponse, Error> {
        let response = self
            .request_xml(Method::POST, path_and_query, input)
            .await
            .context("request_xml")?;

        let push_response = Self::handle_push_response(response).context("handle_push_response")?;

        Ok(PostResponse {
            reboot_required: push_response.reboot_required,
            id: push_response
                .id
                .ok_or_else(|| anyhow!("id missing in response"))?,
        })
    }
    pub async fn put_xml(
        &self,
        path_and_query: PathAndQuery,
        input: Option<Element>,
    ) -> Result<PutResponse, Error> {
        let response = self
            .request_xml(Method::PUT, path_and_query, input)
            .await
            .context("request_xml")?;

        let push_response = Self::handle_push_response(response).context("handle_push_response")?;

        ensure!(
            push_response.id.is_none(),
            "id field present in put response"
        );

        Ok(PutResponse {
            reboot_required: push_response.reboot_required,
        })
    }
    pub async fn delete_xml(
        &self,
        path_and_query: PathAndQuery,
    ) -> Result<DeleteResponse, Error> {
        let response = self
            .request_xml(Method::DELETE, path_and_query, None)
            .await
            .context("request_xml")?;

        let push_response = Self::handle_push_response(response).context("handle_push_response")?;

        ensure!(
            push_response.id.is_none(),
            "id field present in delete response"
        );

        Ok(DeleteResponse {
            reboot_required: push_response.reboot_required,
        })
    }

    fn model_supported(model: &str) -> bool {
        let result = matches!(model, "DS-2CD2132-I" | "DS-2CD2132F-IS" | "DS-2CD2532F-IS");
        result
    }
    fn firmware_version_supported(firmware_version: &Version) -> bool {
        let supported_versions = vec![VersionReq::parse("^5.2.0").unwrap()];
        supported_versions
            .iter()
            .any(|supported_version| supported_version.matches(firmware_version))
    }

    pub async fn validate_basic_device_info(&self) -> Result<BasicDeviceInfo, Error> {
        let device_info_element = self
            .request_xml(
                Method::GET,
                "/ISAPI/System/deviceInfo".parse().unwrap(),
                None,
            )
            .await
            .context("request_xml")?;

        ensure!(
            device_info_element.name == "DeviceInfo",
            "DeviceInfo expected at root level"
        );

        let model: String = device_info_element
            .get_child("model")
            .ok_or_else(|| anyhow!("missing model"))?
            .get_text()
            .ok_or_else(|| anyhow!("missing model text"))?
            .parse()
            .context("model")?;

        ensure!(
            Self::model_supported(&model),
            "this model ({}) is not supported",
            model
        );

        let firmware_version = device_info_element
            .get_child("firmwareVersion")
            .ok_or_else(|| anyhow!("missing firmwareVersion"))?
            .get_text()
            .ok_or_else(|| anyhow!("missing firmwareVersion text"))?
            .strip_prefix('V')
            .ok_or_else(|| anyhow!("missing firmwareVersion prefix"))?
            .parse()
            .context("firmware_version")?;

        ensure!(
            Self::firmware_version_supported(&firmware_version),
            "this firmware version ({}) is not supported",
            &firmware_version
        );

        Ok(BasicDeviceInfo {
            model,
            firmware_version,
        })
    }

    pub async fn snapshot(&self) -> Result<DynamicImage, Error> {
        let content = self
            .request_bytes(
                Method::GET,
                "/ISAPI/Streaming/channels/101/picture".parse().unwrap(),
            )
            .await
            .context("request_bytes")?;

        let content = tokio::task::spawn_blocking(move || -> Result<DynamicImage, Error> {
            let image = image::load_from_memory(&content).context("load_from_memory")?;
            Ok(image)
        })
        .await
        .context("spawn_blocking")??;

        Ok(content)
    }

    pub fn rtsp_url_build(
        &self,
        username: &str,
        password: &str,
        stream: VideoStream,
    ) -> Uri {
        format!(
            "rtsp://{}:{}@{}/Streaming/channels/{}",
            percent_encoding::utf8_percent_encode(username, percent_encoding::NON_ALPHANUMERIC),
            percent_encoding::utf8_percent_encode(password, percent_encoding::NON_ALPHANUMERIC),
            &self.host,
            match stream {
                VideoStream::Main => 101,
                VideoStream::Sub => 102,
            }
        )
        .parse()
        .unwrap()
    }
}

pub struct BoundaryStreamExtractor {
    data_stream: BoxStream<'static, reqwest::Result<Bytes>>,
    data_stream_terminated: bool,
    extractor: boundary_stream::Extractor,
}
impl BoundaryStreamExtractor {
    fn new(data_stream: BoxStream<'static, reqwest::Result<Bytes>>) -> Self {
        let data_stream_terminated = false;
        let extractor = boundary_stream::Extractor::new();
        Self {
            data_stream,
            data_stream_terminated,
            extractor,
        }
    }
}
impl Stream for BoundaryStreamExtractor {
    type Item = Result<Element, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        if !self_.data_stream_terminated {
            match Pin::new(&mut self_.data_stream).poll_next(cx) {
                task::Poll::Ready(Some(item)) => match item.context("item") {
                    Ok(chunk) => match std::str::from_utf8(&chunk).context("from_utf8") {
                        Ok(chunk) => {
                            self_.extractor.push(chunk);
                        }
                        Err(error) => {
                            return task::Poll::Ready(Some(Err(error)));
                        }
                    },
                    Err(error) => {
                        return task::Poll::Ready(Some(Err(error)));
                    }
                },
                task::Poll::Ready(None) => {
                    self_.data_stream_terminated = true;
                }
                task::Poll::Pending => {}
            }
        }

        match self_.extractor.try_extract().context("try_extract") {
            Ok(Some(item)) => task::Poll::Ready(Some(Ok(item))),
            Ok(None) => {
                if self_.data_stream_terminated {
                    task::Poll::Ready(None)
                } else {
                    task::Poll::Pending
                }
            }
            Err(error) => task::Poll::Ready(Some(Err(error))),
        }
    }
}
