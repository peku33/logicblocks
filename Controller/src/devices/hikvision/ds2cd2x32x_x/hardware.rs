use crate::util::atomic_cell::AtomicCell;
use anyhow::{anyhow, bail, ensure, Context, Error};
use bytes::Bytes;
use futures::{
    future::FutureExt,
    pin_mut, select,
    stream::{Stream, StreamExt, TryStreamExt},
};
use http::{
    uri::{self, Authority, PathAndQuery, Scheme},
    Method, Uri,
};
use image::DynamicImage;
use lazy_static::lazy_static;
use regex::Regex;
use semver::{Version, VersionReq};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::DerefMut,
    time::Duration,
};
use tokio::sync::watch;
use xmltree::{Element, XMLNode};

#[derive(Debug)]
struct PushResponse {
    reboot_required: bool,
    id: Option<usize>,
}

#[derive(Debug)]
struct PostResponse {
    reboot_required: bool,
    id: usize,
}
#[derive(Debug)]
struct PutResponse {
    reboot_required: bool,
}
#[derive(Debug)]
struct DeleteResponse {
    reboot_required: bool,
}

#[derive(Debug)]
pub struct BasicDeviceInfo {
    model: String,
    firmware_version: Version,
}

#[derive(Debug)]
pub enum VideoStream {
    MAIN,
    SUB,
}

#[derive(Debug)]
pub struct Client {
    host: Authority,
    admin_password: String,

    reqwest_client: reqwest::Client,
}
impl Client {
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

    async fn request_bytes(
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
    async fn request_xml(
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
    async fn request_mixed_stream(
        &self,
        path_and_query: PathAndQuery,
    ) -> Result<impl Stream<Item = reqwest::Result<Bytes>>, Error> {
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

        let headers = response.headers();
        ensure!(
            headers.get(http::header::CONTENT_TYPE).contains(
                &http::header::HeaderValue::from_static("multipart/mixed; boundary=boundary")
            ),
            "invalid content type for mixed stream"
        );

        let stream = response.bytes_stream();

        Ok(stream)
    }

    async fn get_xml(
        &self,
        path_and_query: PathAndQuery,
    ) -> Result<Element, Error> {
        let response = self
            .request_xml(Method::GET, path_and_query, None)
            .await
            .context("request_xml")?;

        Ok(response)
    }
    async fn post_xml(
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
    async fn put_xml(
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
    async fn delete_xml(
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

    fn model_supported(model: &str) -> Result<bool, Error> {
        #[allow(clippy::match_like_matches_macro)]
        let result = match model {
            "DS-2CD2132F-IS" => true,
            _ => false,
        };
        Ok(result)
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
            Self::model_supported(&model).context("model_supported")?,
            "this model ({}) is not supported",
            model
        );

        let firmware_version = device_info_element
            .get_child("firmwareVersion")
            .ok_or_else(|| anyhow!("missing firmwareVersion"))?
            .get_text()
            .ok_or_else(|| anyhow!("missing firmwareVersion text"))?
            .strip_prefix("V")
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
                VideoStream::MAIN => 101,
                VideoStream::SUB => 102,
            }
        )
        .parse()
        .unwrap()
    }
}

#[derive(Debug)]
pub struct MotionDetectionGrid {
    rows_cols: [[bool; Self::COLUMNS]; Self::ROWS],
}
impl MotionDetectionGrid {
    pub const COLUMNS: usize = 22;
    pub const ROWS: usize = 18;

    pub fn new(rows_cols: [[bool; Self::COLUMNS]; Self::ROWS]) -> Self {
        Self { rows_cols }
    }
    pub fn full() -> Self {
        Self {
            rows_cols: [[true; Self::COLUMNS]; Self::ROWS],
        }
    }

    fn serialize(&self) -> String {
        let mut result = String::with_capacity(24 * 18 * 2 / 8);
        for row in self.rows_cols.iter() {
            for chunk in row.chunks(8) {
                let mut mask: u8 = 0;
                for (offset, bit) in chunk.iter().enumerate() {
                    if *bit {
                        mask |= 1 << (7 - offset);
                    }
                }
                let mut mask_hex = [0u8; 2];
                hex::encode_to_slice(&[mask], &mut mask_hex[..]).unwrap();
                result.push(mask_hex[0] as char);
                result.push(mask_hex[1] as char);
            }
        }
        result
    }
}
#[cfg(test)]
pub mod motion_detection_grid_tests {
    use super::MotionDetectionGrid;

    #[test]
    fn test_full() {
        let motion_detection_grid = MotionDetectionGrid::full();
        assert_eq!(motion_detection_grid.serialize(), "fffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffcfffffc");
    }
    #[test]
    fn test_docs_example() {
        let mut rows_cols = [[false; MotionDetectionGrid::COLUMNS]; MotionDetectionGrid::ROWS];
        rows_cols[6][8] = true;
        rows_cols[6][9] = true;
        rows_cols[6][10] = true;

        rows_cols[7][8] = true;
        rows_cols[7][9] = true;
        rows_cols[7][10] = true;
        rows_cols[7][11] = true;

        rows_cols[8][8] = true;
        rows_cols[8][9] = true;
        rows_cols[8][10] = true;
        rows_cols[8][11] = true;

        rows_cols[9][8] = true;
        rows_cols[9][9] = true;
        rows_cols[9][10] = true;
        rows_cols[9][11] = true;

        rows_cols[14][0] = true;
        rows_cols[14][1] = true;
        rows_cols[14][2] = true;
        rows_cols[14][3] = true;

        rows_cols[15][0] = true;
        rows_cols[15][1] = true;
        rows_cols[15][2] = true;
        rows_cols[15][3] = true;

        rows_cols[16][0] = true;
        rows_cols[16][1] = true;
        rows_cols[16][2] = true;
        rows_cols[16][3] = true;

        rows_cols[17][0] = true;
        rows_cols[17][1] = true;
        rows_cols[17][2] = true;
        rows_cols[17][3] = true;

        let motion_detection_grid = MotionDetectionGrid::new(rows_cols);
        assert_eq!(motion_detection_grid.serialize(), "00000000000000000000000000000000000000e00000f00000f00000f000000000000000000000000000f00000f00000f00000f00000");
    }
}

#[derive(Debug)]
pub enum MotionDetectionSensitivity {
    P00,
    P20,
    P40,
    P60,
    P80,
    P100,
}

#[derive(Debug)]
pub struct MotionDetection {
    pub grid: MotionDetectionGrid,
    pub sensitivity: MotionDetectionSensitivity,
    pub sampling_interval: usize,  // defaults to 2
    pub trigger_start_msec: usize, // defaults to 500
    pub trigger_end_msec: usize,   // defaults to 500
}

#[derive(Debug)]
pub struct Configuration {
    pub device_name: String,
    pub device_id: u8,
    pub overlay_text: String,
    pub shared_user_password: String,
    pub motion_detection: Option<MotionDetection>,
}

pub struct Configurator<'c> {
    client: &'c Client,
}
impl<'c> Configurator<'c> {
    pub const SHARED_USER_LOGIN: &'static str = "logicblocks";

    pub fn new(client: &'c Client) -> Self {
        Self { client }
    }

    pub async fn healthcheck(&mut self) -> Result<(), Error> {
        self.client
            .validate_basic_device_info()
            .await
            .context("basic_device_info")?;
        Ok(())
    }

    async fn wait_for_power_down(&mut self) -> Result<(), Error> {
        for _ in 0..90 {
            if self.healthcheck().await.is_err() {
                return Ok(());
            }
            tokio::time::delay_for(Duration::from_secs(1)).await;
        }
        bail!("device didn't went away in designated time");
    }
    async fn wait_for_power_up(&mut self) -> Result<(), Error> {
        for _ in 0..60 {
            if self.healthcheck().await.is_ok() {
                return Ok(());
            }
            tokio::time::delay_for(Duration::from_secs(1)).await;
        }
        // TODO: Return last failure
        bail!("device didn't went up in designated time");
    }
    pub async fn reboot(&mut self) -> Result<(), Error> {
        self.client
            .put_xml("/ISAPI/System/reboot".parse().unwrap(), None)
            .await
            .context("put_xml")?;
        Ok(())
    }
    pub async fn reboot_wait_for_ready(&mut self) -> Result<(), Error> {
        self.reboot().await.context("reboot")?;

        self.wait_for_power_down()
            .await
            .context("wait_for_power_down")?;

        self.wait_for_power_up()
            .await
            .context("wait_for_power_up")?;

        Ok(())
    }

    async fn system_factory_reset(&mut self) -> Result<(), Error> {
        let mut reboot_required = false;

        reboot_required |= self
            .client
            .put_xml(
                "/ISAPI/System/factoryReset?mode=basic".parse().unwrap(),
                None,
            )
            .await
            .context("Factory reset")?
            .reboot_required;

        if reboot_required {
            self.reboot_wait_for_ready()
                .await
                .context("reboot_wait_for_ready")?;
        }
        Ok(())
    }
    async fn system_device_name(
        &mut self,
        device_name: &str,
        device_id: u8,
    ) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/System/deviceInfo".parse().unwrap(),
                Some(element_build_children(
                    "DeviceInfo",
                    vec![
                        element_build_text("deviceName", device_name),
                        element_build_text("telecontrolID", &device_id.to_string()),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    async fn system_time_gmt(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/System/time".parse().unwrap(),
                Some(element_build_children(
                    "Time",
                    vec![
                        element_build_text("timeMode", "NTP"),
                        element_build_text("timeZone", "CST+0:00:00"),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    async fn system_time_ntp(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/System/time/ntpServers/1".parse().unwrap(),
                Some(element_build_children(
                    "NTPServer",
                    vec![
                        element_build_text("id", "1"),
                        element_build_text("addressingFormatType", "hostname"),
                        element_build_text("hostName", "pool.ntp.org"),
                        element_build_text("portNo", "123"),
                        element_build_text("synchronizeInterval", "1440"),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    async fn system_shared_user(
        &mut self,
        password: &str,
    ) -> Result<(), Error> {
        // Check if user is already added
        let user_ids = self
            .client
            .get_xml("/ISAPI/Security/users".parse().unwrap())
            .await
            .context("get_xml")?
            .children
            .iter()
            .filter_map(|user_entry| {
                let user_id: usize = user_entry
                    .as_element()?
                    .get_child("id")?
                    .get_text()?
                    .parse()
                    .ok()?;

                let user_name = user_entry.as_element()?.get_child("userName")?.get_text()?;

                if user_name == Self::SHARED_USER_LOGIN {
                    Some(user_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // If so - delete
        for user_id in user_ids {
            let reboot_required = self
                .client
                .delete_xml(
                    format!("/ISAPI/Security/users/{}", user_id)
                        .parse()
                        .unwrap(),
                )
                .await
                .context("delete_xml")?
                .reboot_required;
            ensure!(!reboot_required, "reboot is not supported here");
        }

        // Create new user
        let post_result = self
            .client
            .post_xml(
                "/ISAPI/Security/users".parse().unwrap(),
                Some(element_build_children(
                    "User",
                    vec![
                        element_build_text("userName", Self::SHARED_USER_LOGIN),
                        element_build_text("password", password),
                    ],
                )),
            )
            .await
            .context("post_xml")?;
        ensure!(!post_result.reboot_required, "reboot is not supported here");

        // Set user permissions
        let reboot_required = self
            .client
            .put_xml(
                format!("/ISAPI/Security/UserPermission/{}", post_result.id)
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "UserPermission",
                    vec![
                        element_build_text("userID", &post_result.id.to_string()),
                        element_build_text("userType", "viewer"),
                        element_build_children(
                            "remotePermission",
                            vec![element_build_text("preview", "true")],
                        ),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    async fn network_upnp_sane(
        &mut self,
        device_name: &str,
    ) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/System/Network/UPnP".parse().unwrap(),
                Some(element_build_children(
                    "UPnP",
                    vec![
                        element_build_text("enabled", "true"),
                        element_build_text("name", device_name),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    async fn network_port_mapping_disable(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/System/Network/UPnP/ports".parse().unwrap(),
                Some(element_build_children(
                    "ports",
                    vec![
                        element_build_text("enabled", "false"),
                        element_build_text("mapmode", "auto"),
                        element_build_children("portList", vec![]),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    async fn network_ezviz_disable(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/System/Network/EZVIZ".parse().unwrap(),
                Some(element_build_children(
                    "EZVIZ",
                    vec![element_build_text("enabled", "false")],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    async fn video_main_quality(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/Streaming/channels/101".parse().unwrap(),
                Some(element_build_children(
                    "StreamingChannel",
                    vec![
                        element_build_text("id", "101"),
                        element_build_children(
                            "Video",
                            vec![
                                element_build_text("videoResolutionWidth", "2048"),
                                element_build_text("videoResolutionHeight", "1536"),
                                element_build_text("videoQualityControlType", "VBR"),
                                element_build_text("fixedQuality", "100"),
                                element_build_text("vbrUpperCap", "8192"),
                                element_build_text("maxFrameRate", "2000"),
                            ],
                        ),
                        element_build_children(
                            "Audio",
                            vec![element_build_text("enabled", "true")],
                        ),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    async fn video_sub_quality(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/Streaming/channels/102".parse().unwrap(),
                Some(element_build_children(
                    "StreamingChannel",
                    vec![
                        element_build_text("id", "102"),
                        element_build_children(
                            "Video",
                            vec![
                                element_build_text("videoResolutionWidth", "320"),
                                element_build_text("videoResolutionHeight", "240"),
                                element_build_text("videoQualityControlType", "VBR"),
                                element_build_text("fixedQuality", "60"),
                                element_build_text("vbrUpperCap", "256"),
                                element_build_text("maxFrameRate", "2000"),
                            ],
                        ),
                        element_build_children(
                            "Audio",
                            vec![element_build_text("enabled", "true")],
                        ),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    async fn audio(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/System/TwoWayAudio/channels/1".parse().unwrap(),
                Some(element_build_children(
                    "TwoWayAudioChannel",
                    vec![
                        element_build_text("id", "1"),
                        element_build_text("enabled", "true"),
                        element_build_text("audioInputType", "MicIn"),
                        element_build_text("speakerVolume", "100"),
                        element_build_text("noisereduce", "true"),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    async fn image_overlay_text(
        &mut self,
        name: &str,
    ) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1".parse().unwrap(),
                Some(element_build_children(
                    "VideoInputChannel",
                    vec![
                        element_build_text("id", "1"),
                        element_build_text("inputPort", "1"),
                        element_build_text("name", name),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    async fn image_overlay_date(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1/overlays"
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "VideoOverlay",
                    vec![element_build_children(
                        "DateTimeOverlay",
                        vec![
                            element_build_text("enabled", "true"),
                            element_build_text("positionX", "0"),
                            element_build_text("positionY", "544"),
                            element_build_text("dateStyle", "YYYY-MM-DD"),
                            element_build_text("timeStyle", "24hour"),
                            element_build_text("displayWeek", "false"),
                        ],
                    )],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    async fn record_schedule_disable(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .client
            .put_xml(
                "/ISAPI/ContentMgmt/record/tracks/101".parse().unwrap(),
                Some(element_build_children(
                    "Track",
                    vec![
                        element_build_text("id", "101"),
                        element_build_text("Channel", "101"),
                        element_build_text("Enable", "false"),
                        element_build_text("LoopEnable", "true"),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    pub async fn detection_motion_enable(
        &mut self,
        settings: &MotionDetection,
    ) -> Result<(), Error> {
        let mut reboot_required = false;

        reboot_required |= self
            .client
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1/motionDetection"
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "MotionDetection",
                    vec![
                        element_build_text("enabled", "true"),
                        element_build_text("enableHighlight", "false"),
                        element_build_text(
                            "samplingInterval",
                            &settings.sampling_interval.to_string(),
                        ),
                        element_build_text(
                            "startTriggerTime",
                            &settings.trigger_start_msec.to_string(),
                        ),
                        element_build_text(
                            "endTriggerTime",
                            &settings.trigger_end_msec.to_string(),
                        ),
                        element_build_text("regionType", "grid"),
                        element_build_children(
                            "MotionDetectionLayout",
                            vec![
                                element_build_text(
                                    "sensitivityLevel",
                                    &match settings.sensitivity {
                                        MotionDetectionSensitivity::P00 => 0,
                                        MotionDetectionSensitivity::P20 => 20,
                                        MotionDetectionSensitivity::P40 => 40,
                                        MotionDetectionSensitivity::P60 => 60,
                                        MotionDetectionSensitivity::P80 => 80,
                                        MotionDetectionSensitivity::P100 => 100,
                                    }
                                    .to_string(),
                                ),
                                element_build_children(
                                    "layout",
                                    vec![element_build_text("gridMap", &settings.grid.serialize())],
                                ),
                            ],
                        ),
                    ],
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");
        Ok(())
    }
    pub async fn detection_tamper_enable(&mut self) -> Result<(), Error> {
        let mut reboot_required = false;

        reboot_required |= self
            .client
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1/tamperDetection"
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "TamperDetection",
                    vec![
                        element_build_text("enabled", "true"),
                        element_build_children(
                            "TamperDetectionRegionList",
                            vec![element_build_children(
                                "TamperDetectionRegion",
                                vec![
                                    element_build_text("id", "1"),
                                    element_build_text("enabled", "true"),
                                    element_build_text("sensitivityLevel", "100"),
                                    element_build_children(
                                        "RegionCoordinatesList",
                                        vec![
                                            element_build_children(
                                                "RegionCoordinates",
                                                vec![
                                                    element_build_text("positionX", "0"),
                                                    element_build_text("positionY", "0"),
                                                ],
                                            ),
                                            element_build_children(
                                                "RegionCoordinates",
                                                vec![
                                                    element_build_text("positionX", "704"),
                                                    element_build_text("positionY", "0"),
                                                ],
                                            ),
                                            element_build_children(
                                                "RegionCoordinates",
                                                vec![
                                                    element_build_text("positionX", "704"),
                                                    element_build_text("positionY", "576"),
                                                ],
                                            ),
                                            element_build_children(
                                                "RegionCoordinates",
                                                vec![
                                                    element_build_text("positionX", "0"),
                                                    element_build_text("positionY", "576"),
                                                ],
                                            ),
                                        ],
                                    ),
                                ],
                            )],
                        ),
                    ],
                )),
            )
            .await
            .context("factory reset")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");
        Ok(())
    }

    pub async fn configure(
        &mut self,
        configuration: &Configuration,
    ) -> Result<(), Error> {
        // TODO: Progress callback

        self.system_factory_reset()
            .await
            .context("system_factory_reset")?;

        self.system_device_name(&configuration.device_name, configuration.device_id)
            .await
            .context("system_device_name")?;

        self.system_time_gmt() // break
            .await
            .context("system_time_gmt")?;

        self.system_time_ntp() // break
            .await
            .context("system_time_ntp")?;

        self.system_shared_user(&configuration.shared_user_password)
            .await
            .context("system_shared_user")?;

        self.network_upnp_sane(&configuration.device_name)
            .await
            .context("network_upnp_sane")?;

        self.network_port_mapping_disable()
            .await
            .context("network_port_mapping_disable")?;

        self.network_ezviz_disable()
            .await
            .context("network_ezviz_disable")?;

        self.video_main_quality()
            .await
            .context("video_main_quality")?;

        self.video_sub_quality()
            .await
            .context("video_sub_quality")?;

        self.audio() // break
            .await
            .context("audio")?;

        self.image_overlay_text(&configuration.overlay_text)
            .await
            .context("image_overlay_text")?;

        self.image_overlay_date()
            .await
            .context("image_overlay_date")?;

        self.record_schedule_disable()
            .await
            .context("record_schedule_disable")?;

        if let Some(motion_detection) = configuration.motion_detection.as_ref() {
            self.detection_motion_enable(motion_detection)
                .await
                .context("detection_motion_enable")?;
        }

        self.detection_tamper_enable()
            .await
            .context("detection_tamper_enable")?;

        Ok(())
    }
}

fn element_build_text(
    name: &str,
    text: &str,
) -> Element {
    let mut element = Element::new(name);
    element.children.push(XMLNode::Text(text.to_owned()));
    element
}
fn element_build_children(
    name: &str,
    children: Vec<Element>,
) -> Element {
    let mut element = Element::new(name);
    element.children = children.into_iter().map(XMLNode::Element).collect();
    element
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum Event {
    CameraFailure,
    VideoLoss,
    TamperingDetection,
    MotionDetection,
    LineDetection,
    FieldDetection,
}
#[derive(Debug)]
pub struct EventStateUpdate {
    event: Event,
    active: bool,
}
pub type Events = HashSet<Event>;

pub struct EventStreamManager<'c> {
    client: &'c Client,
    mixed_content_extractor: AtomicCell<MixedContentExtractor>,
    events_active: AtomicCell<HashMap<Event, usize>>, // Event -> Ticks left

    events_sender: watch::Sender<Events>,
    events_receiver: AtomicCell<watch::Receiver<Events>>,
}
impl<'c> EventStreamManager<'c> {
    const EVENT_STREAM_TIMEOUT: Duration = Duration::from_secs(1);
    const EVENT_DISABLE_TICK_INTERVAL: Duration = Duration::from_millis(250);
    const EVENT_DISABLE_TICKS: usize = 5; // 1250ms
    const ERROR_RESTART_DELAY: Duration = Duration::from_secs(1);

    pub fn new(client: &'c Client) -> Self {
        let (events_sender, events_receiver) = watch::channel(Events::new());

        Self {
            client,
            mixed_content_extractor: AtomicCell::new(MixedContentExtractor::new()),
            events_active: AtomicCell::new(HashMap::new()),

            events_sender,
            events_receiver: AtomicCell::new(events_receiver),
        }
    }

    pub fn receiver(&self) -> impl DerefMut<Target = watch::Receiver<Events>> + '_ {
        self.events_receiver.lease()
    }

    fn parse_event_state_update(element: Element) -> Result<EventStateUpdate, Error> {
        let event_type = element
            .get_child("eventType")
            .ok_or_else(|| anyhow!("missing eventType"))?
            .get_text()
            .ok_or_else(|| anyhow!("missing eventType text"))?;

        let event_state = element
            .get_child("eventState")
            .ok_or_else(|| anyhow!("missing eventState"))?
            .get_text()
            .ok_or_else(|| anyhow!("missing eventState text"))?;

        let event = match event_type.as_ref() {
            "videoloss" => Event::VideoLoss,
            "shelteralarm" => Event::TamperingDetection,
            "VMD" => Event::MotionDetection,
            "linedetection" => Event::LineDetection,
            "fielddetection" => Event::FieldDetection,
            _ => bail!("unknown event type: {}", event_type),
        };
        let active = match event_state.as_ref() {
            "inactive" => false,
            "active" => true,
            _ => bail!("unknown event state: {}", event_state),
        };

        Ok(EventStateUpdate { event, active })
    }
    fn handle_event_state_update(
        &self,
        event_state_update: EventStateUpdate,
    ) -> bool {
        let mut events_active = self.events_active.lease();
        if event_state_update.active {
            events_active
                .insert(event_state_update.event, Self::EVENT_DISABLE_TICKS)
                .is_none()
        } else {
            events_active.remove(&event_state_update.event).is_some()
        }
    }
    fn handle_events_disabler(&self) -> bool {
        let mut events_active = self.events_active.lease();
        events_active
            .drain_filter(|_, ticks_left| {
                *ticks_left -= 1;
                *ticks_left == 0
            })
            .count()
            > 0
    }

    fn propagate_events(&self) {
        let events = self
            .events_active
            .lease()
            .keys()
            .cloned()
            .collect::<Events>();

        self.events_sender.broadcast(events).unwrap();
    }

    pub async fn run_once(&self) -> Error {
        let data_stream = match self
            .client
            .request_mixed_stream("/ISAPI/Event/notification/alertStream".parse().unwrap())
            .await
            .context("request_mixed_stream")
        {
            Ok(stream) => stream,
            Err(error) => return error,
        };
        // TODO: Add timeout
        let data_stream_runner = data_stream
            .err_into::<Error>()
            .try_for_each(async move |chunk| {
                let chunk = std::str::from_utf8(&chunk).context("from_utf8")?;
                let mut mixed_content_extractor = self.mixed_content_extractor.lease();
                mixed_content_extractor.push(chunk);

                let mut events_changed = false;
                for element in mixed_content_extractor.try_extract().into_vec().into_iter() {
                    let event_state_update = Self::parse_event_state_update(element)
                        .context("parse_event_state_update")?;
                    events_changed |= self.handle_event_state_update(event_state_update);
                }
                if events_changed {
                    self.propagate_events();
                }
                Ok(())
            })
            .map(|result| match result {
                Ok(()) => anyhow!("data_stream completed"),
                Err(error) => error,
            });
        pin_mut!(data_stream_runner);
        let mut data_stream_runner = data_stream_runner.fuse();

        let events_disabler_runner = tokio::time::interval(Self::EVENT_DISABLE_TICK_INTERVAL)
            .for_each(async move |_time_point| {
                let mut events_changed = false;
                events_changed |= self.handle_events_disabler();
                if events_changed {
                    self.propagate_events();
                }
            });
        pin_mut!(events_disabler_runner);
        let mut events_disabler_runner = events_disabler_runner.fuse();

        select! {
            data_stream_runner_error = data_stream_runner => data_stream_runner_error,
            _ = events_disabler_runner => panic!("events_disabler_runner"),
        }
    }
    pub async fn run(&self) -> ! {
        loop {
            let error = self.run_once().await.context("run_once");
            log::error!("device {} failed: {:?}", self.client.host, error);
            tokio::time::delay_for(Self::ERROR_RESTART_DELAY).await;
        }
    }
}

struct MixedContentExtractor {
    buffer: VecDeque<u8>,
}
impl MixedContentExtractor {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    pub fn push(
        &mut self,
        chunk: &str,
    ) {
        self.buffer.extend(chunk.bytes());
    }

    pub fn try_extract(&mut self) -> Box<[Element]> {
        lazy_static! {
            static ref PATTERN: Regex = Regex::new("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: (\\d+)\r\n\r\n").unwrap();
        }

        let buffer = self.buffer.make_contiguous();
        let buffer = unsafe { std::str::from_utf8_unchecked(buffer) }; // SAFETY: buffer accepts &str only

        let mut elements = Vec::new();
        let mut start_index: usize = 0;
        while let Some(capture) = PATTERN.captures(&buffer[start_index..]) {
            let header = capture.get(0).unwrap();
            if header.start() != 0 {
                log::warn!("whole.start() != start_index, got some noise on input?");
            }

            let content_length = match capture
                .get(1)
                .unwrap()
                .as_str()
                .parse::<usize>()
                .context("content_length parse")
            {
                Ok(content_length) => content_length,
                Err(error) => {
                    log::warn!("failed to parse content_length: {:?}", error);

                    start_index += header.end(); // Skip header
                    continue;
                }
            };

            // Do we have whole message in buffer?
            if content_length - 1 > buffer.len() - start_index - header.end() {
                break;
            }

            let element = match Element::parse(
                (&buffer[start_index + header.end()..start_index + header.end() + content_length])
                    .as_bytes(),
            ) {
                Ok(element) => element,
                Err(error) => {
                    log::warn!("failed to parse element: {:?}", error);

                    start_index += header.end() + content_length; // Skip payload
                    continue;
                }
            };

            elements.push(element);
            start_index += header.end() + content_length;
        }
        self.buffer.drain(0..start_index);

        elements.into_boxed_slice()
    }
}

#[cfg(test)]
pub mod mixed_content_extractor_tests {
    use super::MixedContentExtractor;

    #[test]
    fn test_1() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_2() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push(
            "--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478",
        );

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);

        extractor.push("\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_3() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);

        extractor.push("<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_4() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);

        extractor.push("\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_5() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 2);

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_6() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);

        extractor.push("\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);
    }
}
