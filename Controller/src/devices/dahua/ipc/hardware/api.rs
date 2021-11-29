use super::boundary_stream;
use anyhow::{anyhow, ensure, Context, Error};
use bytes::Bytes;
use digest_auth::{AuthContext, WwwAuthenticateHeader};
use futures::{
    lock::Mutex,
    stream::{BoxStream, Stream, StreamExt},
};
use http::{
    uri::{self, Authority, PathAndQuery, Scheme},
    Uri,
};
use image::DynamicImage;
use itertools::Itertools;
use md5::{Digest, Md5};
use serde_json::json;
use std::{fmt, pin::Pin, str, sync::atomic::{AtomicU64, Ordering}, task, time::Duration};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct WebVersion {
    pub major: u8,
    pub minor: u8,
    pub revision: u8,
    pub build: usize,
}
impl fmt::Display for WebVersion {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}.{}.{}.{}",
            self.major, self.minor, self.revision, self.build
        )
    }
}

#[derive(Debug)]
pub struct BasicDeviceInfo {
    pub device_type: String,
    pub version: String,
    pub web_version: WebVersion,
    pub serial_number: String,
}

#[derive(Clone, Copy, Debug)]
pub enum VideoStream {
    Main,
    Sub1,
    Sub2,
}

#[derive(Debug)]
struct Rpc2Request {
    method: String,
    params: serde_json::Value,
    session: Option<String>,
}
#[derive(Debug)]
struct Rpc2Response {
    result: bool,
    params: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
    session: Option<String>,
}

#[derive(Debug)]
pub struct Api {
    host: Authority,
    admin_password: String,

    reqwest_client: reqwest::Client,

    rpc2_request_id_next: AtomicU64,
    rpc2_session_cache: Mutex<Option<String>>,
}
impl Api {
    pub fn new(
        host: Authority,
        admin_password: String,
    ) -> Self {
        let reqwest_client = reqwest::ClientBuilder::new().build().unwrap();

        let rpc2_request_id_next = 0;
        let rpc2_request_id_next = AtomicU64::new(rpc2_request_id_next);

        let rpc2_session_cache: Option<String> = None;
        let rpc2_session_cache = Mutex::new(rpc2_session_cache);

        Self {
            host,
            admin_password,

            reqwest_client,

            rpc2_request_id_next,
            rpc2_session_cache,
        }
    }

    // http api with digest auth
    async fn http_request(
        &self,
        mut request: reqwest::Request,
    ) -> Result<reqwest::Response, Error> {
        let mut response = self
            .reqwest_client
            .execute(request.try_clone().unwrap())
            .await
            .context("execute unauthorized")?;

        if response.status() == http::StatusCode::UNAUTHORIZED {
            let www_authenticate_header = response
                .headers()
                .get(http::header::WWW_AUTHENTICATE)
                .ok_or_else(|| anyhow!("got 401, but no www-authenticate?"))?
                .to_str()
                .context("to_str")?;

            // camera does not support context reusing, lol?
            let mut www_authenticate_header =
                WwwAuthenticateHeader::parse(www_authenticate_header).context("parse")?;
            let digest_auth_context =
                AuthContext::new("admin", &self.admin_password, request.url().as_str());
            let authorization_header = www_authenticate_header
                .respond(&digest_auth_context)
                .context("respond")?;

            request.headers_mut().insert(
                http::header::AUTHORIZATION,
                http::HeaderValue::from_str(&authorization_header.to_header_string()).unwrap(),
            );

            response = self
                .reqwest_client
                .execute(request.try_clone().unwrap())
                .await
                .context("execute authorized")?;
        }

        let response = response.error_for_status().context("error_for_status")?;
        Ok(response)
    }

    pub async fn http_request_boundary_stream(
        &self,
        path_and_query: PathAndQuery,
    ) -> Result<BoundaryStreamExtractor, Error> {
        let url = uri::Builder::new()
            .scheme(Scheme::HTTP)
            .authority(self.host.clone())
            .path_and_query(path_and_query)
            .build()
            .unwrap();

        let request = reqwest::Request::new(http::Method::GET, url.to_string().parse().unwrap());

        let response = self.http_request(request).await.context("http_request")?;

        let content_type = response
            .headers()
            .get(http::header::CONTENT_TYPE)
            .ok_or_else(|| anyhow!("missing content type"))?;
        ensure!(content_type == "multipart/x-mixed-replace; boundary=myboundary");

        let data_stream = response.bytes_stream().boxed();

        let boundary_stream_extractor = BoundaryStreamExtractor::new(data_stream);

        Ok(boundary_stream_extractor)
    }

    // rpc2
    const RPC2_TIMEOUT: Duration = Duration::from_secs(10);
    async fn rpc2_request(
        &self,
        path_and_query: PathAndQuery,
        request: Rpc2Request,
    ) -> Result<Rpc2Response, Error> {
        let request_id = self.rpc2_request_id_next.fetch_add(1, Ordering::Relaxed);

        let mut rpc_request = json!({
            "method": request.method,
            "params": request.params,
            "id": request_id,
        });
        if let Some(session) = request.session {
            rpc_request
                .as_object_mut()
                .unwrap()
                .insert("session".to_owned(), serde_json::Value::String(session));
        }
        let rpc_request = rpc_request;

        let url = uri::Builder::new()
            .scheme(Scheme::HTTP)
            .authority(self.host.clone())
            .path_and_query(path_and_query)
            .build()
            .unwrap();

        let response = self
            .reqwest_client
            .post(url.to_string())
            .timeout(Self::RPC2_TIMEOUT)
            .header(http::header::ACCEPT, "application/json")
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(&rpc_request)
            .send()
            .await
            .context("send")?
            .error_for_status()
            .context("error_for_status")?
            .json::<serde_json::Value>()
            .await
            .context("json")?;

        let response = response
            .as_object()
            .ok_or_else(|| anyhow!("object expected"))?;

        // response_id
        // for some responses the id is missing
        if let Some(response_id) = response.get("id") {
            let response_id = response_id
                .as_u64()
                .ok_or_else(|| anyhow!("expected u64"))?;
            ensure!(request_id == response_id);
        }

        // result
        let result = response
            .get("result")
            .ok_or_else(|| anyhow!("missing result"))?
            .as_bool()
            .ok_or_else(|| anyhow!("expected bool"))?;

        // params
        let params = response.get("params").cloned();

        // error
        let error = response.get("error").cloned();

        // session
        // session might be missing or int? not sure why
        let session = response
            .get("session")
            .and_then(|session| session.as_str())
            .map(|session| session.to_owned());

        let response = Rpc2Response {
            result,
            params,
            error,
            session,
        };
        Ok(response)
    }

    async fn rpc2_login_prepare_password(&self) -> Result<(String, String), Error> {
        let request = Rpc2Request {
            method: "global.login".to_owned(),
            params: json!({
                "userName": "admin",
                "password": "",
                "clientType": "Dahua3.0-Web3.0",
            }),
            session: None,
        };

        let response = self
            .rpc2_request("/RPC2_Login".parse().unwrap(), request)
            .await
            .context("rpc2_request")?;

        ensure!(!response.result); // returns false for no reason

        let error = response
            .error
            .as_ref()
            .ok_or_else(|| anyhow!("missing error"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?;

        let code = error
            .get("code")
            .ok_or_else(|| anyhow!("missing code"))?
            .as_u64()
            .ok_or_else(|| anyhow!("expected number"))?;
        ensure!(code == 268632079);

        let params = response
            .params
            .as_ref()
            .ok_or_else(|| anyhow!("missing params"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?;

        let encryption = params
            .get("encryption")
            .ok_or_else(|| anyhow!("missing encryption"))?
            .as_str()
            .ok_or_else(|| anyhow!("expected string"))?;
        ensure!(encryption == "Default");

        let realm = params
            .get("realm")
            .ok_or_else(|| anyhow!("missing realm"))?
            .as_str()
            .ok_or_else(|| anyhow!("expected string"))?;

        let random = params
            .get("random")
            .ok_or_else(|| anyhow!("missing random"))?
            .as_str()
            .ok_or_else(|| anyhow!("expected string"))?;

        let session = response
            .session
            .ok_or_else(|| anyhow!("session missing in response"))?;

        let realm_phase = {
            let mut d = Md5::new();
            d.update("admin");
            d.update(":");
            d.update(realm);
            d.update(":");
            d.update(&self.admin_password);
            let h = d.finalize();
            h
        };
        let realm_phase = hex::encode_upper(realm_phase);

        let random_phase = {
            let mut d = Md5::new();
            d.update("admin");
            d.update(":");
            d.update(random);
            d.update(":");
            d.update(realm_phase);
            let h = d.finalize();
            h
        };
        let random_phase = hex::encode_upper(random_phase);

        Ok((random_phase, session))
    }
    async fn rpc2_login_initialize_session(
        &self,
        password_digest: &str,
        session: &str,
    ) -> Result<u64, Error> {
        let request = Rpc2Request {
            method: "global.login".to_owned(),
            params: json!({
                "userName": "admin",
                "password": password_digest,
                "clientType": "Dahua3.0-Web3.0",
                "authorityType": "Default",
                "passwordType": "Default",
            }),
            session: Some(session.to_owned()),
        };

        let response = self
            .rpc2_request("/RPC2_Login".parse().unwrap(), request)
            .await
            .context("rpc2_request")?;

        ensure!(
            response.result && response.error.is_none(),
            "login failed: params={:?} error={:?}",
            response.params,
            response.error
        );

        let params = response
            .params
            .as_ref()
            .ok_or_else(|| anyhow!("missing params"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?;
        let keep_alive_interval = params
            .get("keepAliveInterval")
            .ok_or_else(|| anyhow!("keep alive interval missing"))?
            .as_u64()
            .ok_or_else(|| anyhow!("expected number"))?;

        ensure!(
            response.session.as_deref() == Some(session),
            "session mismatch"
        );

        Ok(keep_alive_interval)
    }
    async fn rpc2_login(&self) -> Result<(String, u64), Error> {
        let (password_digest, session) = self
            .rpc2_login_prepare_password()
            .await
            .context("rpc2_login_prepare_password")?;
        let session_expiration_seconds = self
            .rpc2_login_initialize_session(&password_digest, &session)
            .await
            .context("rpc2_login_initialize_session")?;
        Ok((session, session_expiration_seconds))
    }

    fn error_is_invalid_session(error: &Option<serde_json::Value>) -> bool {
        let error = match error {
            Some(error) => error,
            None => return false,
        };

        let code = error.get("code").and_then(|code| code.as_u64());
        let message = error.get("message").and_then(|message| message.as_str());

        code.contains(&287637505) && message.contains(&"Invalid session in request data!")
    }
    pub async fn rpc2(
        &self,
        method: impl ToString,
        params: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, Error> {
        const RETRY_COUNT: usize = 3;

        let mut rpc2_session_cache = self.rpc2_session_cache.lock().await;
        let mut retry_id: usize = 0;

        let params = loop {
            retry_id += 1;

            // make sure session exists
            if rpc2_session_cache.is_none() {
                let login = self.rpc2_login().await.context("rpc2_login")?;
                *rpc2_session_cache = Some(login.0);
            }
            let session = rpc2_session_cache.as_ref().unwrap();

            // try making the request
            let request = Rpc2Request {
                method: method.to_string(),
                params: params.clone(),
                session: Some(session.to_owned()),
            };
            let response = self
                .rpc2_request("/RPC2".parse().unwrap(), request)
                .await
                .context("rpc2_request")?;

            // if error means invalid session, retry
            if retry_id < RETRY_COUNT && Self::error_is_invalid_session(&response.error) {
                *rpc2_session_cache = None;
                continue;
            }

            ensure!(
                response.result && response.error.is_none(),
                "request failed: params={:?} error={:?}",
                response.params,
                response.error
            );
            ensure!(response.session.as_deref() == Some(session));

            // if succeeds - break
            break response.params;
        };

        Ok(params)
    }

    // procedures
    fn parse_web_version_string(version: &str) -> Result<WebVersion, Error> {
        let version = version.strip_prefix('V').unwrap_or(version);
        let (major, minor, revision, build) = version
            .split('.')
            .collect_tuple()
            .ok_or_else(|| anyhow!("invalid version string"))?;

        let major = major.parse::<u8>()?;
        let minor = minor.parse::<u8>()?;
        let revision = revision.parse::<u8>()?;
        let build = build.parse::<usize>()?;

        Ok(WebVersion {
            major,
            minor,
            revision,
            build,
        })
    }
    fn device_type_supported(device_type: &str) -> bool {
        matches!(device_type, "IPC-HDW4631C-A")
    }
    fn web_version_supported(web_version: &WebVersion) -> bool {
        matches!(
            web_version,
            WebVersion {
                major: 3,
                minor: 2,
                revision: 1,
                build: 561950,
            } | WebVersion {
                major: 3,
                minor: 2,
                revision: 1,
                build: 582554,
            }
        )
    }
    pub async fn validate_basic_device_info(&self) -> Result<BasicDeviceInfo, Error> {
        let device_type = self
            .rpc2("magicBox.getDeviceType", serde_json::Value::Null)
            .await
            .context("rpc2")?;
        let device_type = device_type
            .ok_or_else(|| anyhow!("missing params"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("type")
            .ok_or_else(|| anyhow!("missing type"))?
            .as_str()
            .ok_or_else(|| anyhow!("expected string"))?
            .to_owned();
        ensure!(
            Self::device_type_supported(&device_type),
            "this device type ({}) is not supported",
            &device_type
        );

        let software_version = self
            .rpc2("magicBox.getSoftwareVersion", serde_json::Value::Null)
            .await
            .context("rpc2")?;
        let software_version = software_version
            .as_ref()
            .ok_or_else(|| anyhow!("missing params"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("version")
            .ok_or_else(|| anyhow!("missing version"))?;
        let version = software_version
            .get("Version")
            .ok_or_else(|| anyhow!("missing version"))?
            .as_str()
            .ok_or_else(|| anyhow!("expected string"))?
            .to_owned();
        let web_version = software_version
            .get("WebVersion")
            .ok_or_else(|| anyhow!("missing web version"))?
            .as_str()
            .ok_or_else(|| anyhow!("expected string"))?;
        let web_version = Self::parse_web_version_string(web_version)?;
        ensure!(
            Self::web_version_supported(&web_version),
            "this version ({}) is not supported",
            &web_version
        );

        let serial_number = self
            .rpc2("magicBox.getSerialNo", serde_json::Value::Null)
            .await
            .context("rpc2")?;
        let serial_number = serial_number
            .ok_or_else(|| anyhow!("missing params"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("sn")
            .ok_or_else(|| anyhow!("missing sn"))?
            .as_str()
            .ok_or_else(|| anyhow!("expected string"))?
            .to_owned();

        let basic_device_info = BasicDeviceInfo {
            device_type,
            version,
            web_version,
            serial_number,
        };

        Ok(basic_device_info)
    }

    const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);
    pub async fn snapshot(&self) -> Result<DynamicImage, Error> {
        let url = uri::Builder::new()
            .scheme(Scheme::HTTP)
            .authority(self.host.clone())
            .path_and_query("/cgi-bin/snapshot.cgi")
            .build()
            .unwrap();

        let mut request =
            reqwest::Request::new(http::Method::GET, url.to_string().parse().unwrap());
        request
            .headers_mut()
            .insert(http::header::ACCEPT, "image/jpeg".parse().unwrap());
        *request.timeout_mut() = Some(Self::SNAPSHOT_TIMEOUT);

        let response = self
            .http_request(request)
            .await
            .context("http_request")?
            .bytes()
            .await
            .context("bytes")?;

        let content = tokio::task::spawn_blocking(move || -> Result<DynamicImage, Error> {
            let image = image::load_from_memory(&response).context("load_from_memory")?;
            Ok(image)
        })
        .await
        .context("spawn_blocking")??;

        Ok(content)
    }

    const SNAPSHOT_RETRY_INTERVAL: Duration = Duration::from_secs(5);
    pub async fn snapshot_retry(
        &self,
        retries_max: usize,
    ) -> Result<DynamicImage, Error> {
        let mut retries_left = retries_max;
        loop {
            let result = self.snapshot().await.context("snapshot");
            if let Err(error) = result.as_ref() {
                log::warn!("error while getting snapshot: {:?}", error);
            }
            if result.is_ok() || retries_left == 0 {
                return result;
            }
            tokio::time::sleep(Self::SNAPSHOT_RETRY_INTERVAL).await;
            retries_left -= 1;
        }
    }

    pub fn rtsp_url_build(
        &self,
        username: &str,
        password: &str,
        stream: VideoStream,
    ) -> Uri {
        format!(
            "rtsp://{}:{}@{}/cam/realmonitor?channel=1&subtype={}",
            percent_encoding::utf8_percent_encode(username, percent_encoding::NON_ALPHANUMERIC),
            percent_encoding::utf8_percent_encode(password, percent_encoding::NON_ALPHANUMERIC),
            &self.host,
            match stream {
                VideoStream::Main => 0,
                VideoStream::Sub1 => 1,
                VideoStream::Sub2 => 2,
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
    type Item = Result<String, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        let self_ = unsafe { self.get_unchecked_mut() };

        if !self_.data_stream_terminated {
            match Pin::new(&mut self_.data_stream).poll_next(cx) {
                task::Poll::Ready(Some(item)) => match item.context("item") {
                    Ok(chunk) => match str::from_utf8(&chunk).context("from_utf8") {
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
