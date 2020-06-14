use bytes::Bytes;
use failure::{err_msg, format_err, Error};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

pub const SHARED_USER_NAME: &str = "logicblocks";

#[derive(Default, Clone, Debug)]
pub struct SaneDefaultsConfig {
    pub device_name: String,
    pub shared_user_password: String,
    pub video_overlay: Option<String>,
}

#[derive(Copy, Clone, Debug)]
pub enum Stream {
    Main = 0,
    Sub1 = 1,
    Sub2 = 2,
}

#[derive(Debug)]
pub struct Client {
    host: http::uri::Authority,
    admin_password: String,

    reqwest_client: reqwest::Client,
}
impl Client {
    pub fn new(
        host: http::uri::Authority,
        admin_password: String,
    ) -> Self {
        Self {
            host,
            admin_password,

            reqwest_client: reqwest::Client::new(),
        }
    }
    async fn _request(
        &self,
        endpoint: http::uri::PathAndQuery,
    ) -> Result<reqwest::Response, Error> {
        let url = http::uri::Builder::new()
            .scheme(http::uri::Scheme::HTTP)
            .authority(self.host.clone())
            .path_and_query(endpoint.clone())
            .build()?
            .to_string();

        let response = self.reqwest_client.get(&url).send().await?;

        if response.status().is_success() {
            return Ok(response);
        } else if response.status() != http::StatusCode::UNAUTHORIZED {
            return Err(format_err!(
                "{} expected in authentication phase, but received {}",
                http::StatusCode::UNAUTHORIZED,
                response.status()
            ));
        }

        let response = self
            .reqwest_client
            .get(&url)
            .header(
                http::header::AUTHORIZATION,
                digest_auth::parse(
                    response
                        .headers()
                        .get(http::header::WWW_AUTHENTICATE)
                        .ok_or_else(|| {
                            err_msg("WWW_AUTHENTICATE header missing during initial phase")
                        })?
                        .to_str()?,
                )?
                .respond(&digest_auth::AuthContext::new(
                    "admin",
                    &self.admin_password,
                    &url,
                ))?
                .to_header_string(),
            )
            .send()
            .await?
            .error_for_status()?;

        Ok(response)
    }
    async fn _request_text(
        &self,
        endpoint: http::uri::PathAndQuery,
    ) -> Result<String, Error> {
        let response_text = self._request(endpoint).await?.text().await?;

        Ok(response_text)
    }
    async fn _request_bytes(
        &self,
        endpoint: http::uri::PathAndQuery,
    ) -> Result<Bytes, Error> {
        let response_bytes = self._request(endpoint).await?.bytes().await?;

        Ok(response_bytes)
    }
    async fn _request_text_ok(
        &self,
        endpoint: http::uri::PathAndQuery,
    ) -> Result<(), Error> {
        let response = self._request_text(endpoint).await?;
        if response != "OK\r\n" {
            return Err(format_err!(
                "invalid response: '{}', expecting 'OK'",
                response
            ));
        }

        Ok(())
    }
    async fn _request_text_parse_table(
        &self,
        endpoint: http::uri::PathAndQuery,
    ) -> Result<HashMap<String, String>, Error> {
        fn _map_table_line(line: &str) -> Result<(&str, &str), Error> {
            lazy_static! {
                static ref R: Regex = Regex::new(r"^([\w\[\]\.]+)=([\w\.]+)$").unwrap();
            }

            let c = R.captures(line);
            let c = match c {
                Some(c) => c,
                None => return Err(format_err!("line {} didn't match table pattern", line)),
            };

            let key = c
                .get(1)
                .ok_or_else(|| err_msg("missing capture group 1"))?
                .as_str();
            let value = c
                .get(2)
                .ok_or_else(|| err_msg("missing capture group 2"))?
                .as_str();

            Ok((key, value))
        }

        self._request_text(endpoint)
            .await?
            .lines()
            .map(|line| _map_table_line(line).map(|(key, value)| (key.into(), value.into())))
            .collect()
    }

    async fn _wait_for_power_down(&self) -> Result<usize, Error> {
        for retry_id in 0..90 {
            if self.healthcheck().await.is_err() {
                return Ok(retry_id);
            }
            tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
        }
        Err(err_msg("device didn't went away in designated time"))
    }
    async fn _wait_for_power_up(&self) -> Result<usize, Error> {
        for retry_id in 0..60 {
            if self.healthcheck().await.is_ok() {
                return Ok(retry_id);
            }
            tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
        }
        // TODO: Return last failure
        Err(err_msg("device didn't went up in designated time"))
    }
    async fn _wait_for_power_down_up(&self) -> Result<(), Error> {
        self._wait_for_power_down().await?;
        self._wait_for_power_up().await?;

        Ok(())
    }

    pub fn get_stream_rtsp_uri(
        &self,
        stream: Stream,
        shared_user_password: &str,
    ) -> url::Url {
        format!(
            "rtsp://{}:{}@{}/cam/realmonitor?channel=1&subtype={}",
            percent_encoding::utf8_percent_encode(
                SHARED_USER_NAME,
                percent_encoding::NON_ALPHANUMERIC
            ),
            percent_encoding::utf8_percent_encode(
                shared_user_password,
                percent_encoding::NON_ALPHANUMERIC
            ),
            &self.host,
            stream as usize
        )
        .parse()
        .unwrap()
    }

    pub async fn healthcheck(&self) -> Result<(), Error> {
        self._request_text(
            "/cgi-bin/magicBox.cgi?action=getDeviceType"
                .parse()
                .unwrap(),
        )
        .await?;

        Ok(())
    }
    pub async fn snapshot(&self) -> Result<image::DynamicImage, Error> {
        let data = self
            ._request_bytes("/cgi-bin/snapshot.cgi".parse().unwrap())
            .await?;

        let snapshot_image = image::load_from_memory(&data)?;

        Ok(snapshot_image)
    }
    pub async fn defaults_except_network(&self) -> Result<(), Error> {
        self._request_text_ok(
            "/cgi-bin/configManager.cgi?action=restoreExcept&names[0]=Network"
                .parse()
                .unwrap(),
        )
        .await?;

        Ok(())
    }
    pub async fn reboot(&self) -> Result<(), Error> {
        self._request_text_ok("/cgi-bin/magicBox.cgi?action=reboot".parse().unwrap())
            .await?;

        Ok(())
    }
    pub async fn reboot_wait_for_ready(&self) -> Result<(), Error> {
        self.reboot().await?;
        self._wait_for_power_down_up().await?;

        Ok(())
    }

    pub async fn sane_defaults(
        &self,

        sane_defaults_config: &SaneDefaultsConfig,
    ) -> Result<(), Error> {
        log::trace!("{:?}: checking device", self.host);
        self.healthcheck().await?;

        log::trace!(
            "{:?}: restore factory settings (Except networking)",
            self.host
        );
        if let Err(error) = self.defaults_except_network().await {
            log::warn!(
                "{:?}: error while resetting to factory settings, this is likely false positive (device bug): {:?}",
                self.host,
                error
            );
        }

        if let Err(error) = self._wait_for_power_down_up().await {
            log::warn!(
                "{:?}: error while waiting for reboot: {:?}",
                self.host,
                error
            );
        }

        log::trace!("{:?}: set device name", self.host);
        self._request_text_ok(
            format!(
                "\
                 /cgi-bin/configManager.cgi?action=setConfig&\
                 General.MachineName={}\
                 ",
                percent_encoding::utf8_percent_encode(
                    &sane_defaults_config.device_name,
                    percent_encoding::NON_ALPHANUMERIC
                )
            )
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: set TCP/IP hostname", self.host);
        self._request_text_ok(
            format!(
                "\
                 /cgi-bin/configManager.cgi?action=setConfig&\
                 Network.Hostname={}\
                 ",
                percent_encoding::utf8_percent_encode(
                    &sane_defaults_config.device_name,
                    percent_encoding::NON_ALPHANUMERIC
                )
            )
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: set GMT+0 timezone, NTP", self.host);
        // Fucks up in case of combined requests
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             NTP.TimeZone=0\
             "
            .parse()
            .unwrap(),
        )
        .await?;
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             NTP.UpdatePeriod=30\
             "
            .parse()
            .unwrap(),
        )
        .await?;
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             NTP.Address=pool.ntp.org\
             "
            .parse()
            .unwrap(),
        )
        .await?;
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             NTP.Enable=true\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: disable multicast", self.host);
        // Fails if pushed together
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             Multicast.DHII[0].Enable=false\
             "
            .parse()
            .unwrap(),
        )
        .await?;
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             Multicast.RTP[0].Enable=false\
             "
            .parse()
            .unwrap(),
        )
        .await?;
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             Multicast.RTP[1].Enable=false\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: disable Easy4Ip", self.host);
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             T2UServer.Enable=false\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: disable Bonjour", self.host);
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             Bonjour.Enable=false\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: disable Lechange Pro", self.host);
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             VSP_PaaS.Enable=false\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: set up shared user", self.host);
        // This call may fail if user does not exist
        let _ = self
            ._request_text_ok(
                format!(
                    "\
                     /cgi-bin/userManager.cgi?action=deleteUser&\
                     name={}\
                     ",
                    percent_encoding::utf8_percent_encode(
                        SHARED_USER_NAME,
                        percent_encoding::NON_ALPHANUMERIC
                    )
                )
                .parse()
                .unwrap(),
            )
            .await;
        self._request_text_ok(
            format!(
                "\
                 /cgi-bin/userManager.cgi?action=addUser&\
                 user.Name={}&\
                 user.Password={}&\
                 user.Group=user&\
                 user.Reserved=false&\
                 user.Sharable=true&\
                 user.AuthList=\
                 ",
                percent_encoding::utf8_percent_encode(
                    SHARED_USER_NAME,
                    percent_encoding::NON_ALPHANUMERIC
                ),
                percent_encoding::utf8_percent_encode(
                    &sane_defaults_config.shared_user_password,
                    percent_encoding::NON_ALPHANUMERIC
                )
            )
            .parse()
            .unwrap(),
        )
        .await?;

        // FIXME: Breaks audio mutation
        // log::trace!("{:?}: disable storage", self.host);
        // self._request_text_ok(
        //     "\
        //      /cgi-bin/configManager.cgi?action=setConfig&\
        //      RecordStoragePoint[0].AlarmRecord.FTP=false&\
        //      RecordStoragePoint[0].AlarmSnapShot.FTP=false&\
        //      RecordStoragePoint[0].EventRecord.FTP=false&\
        //      RecordStoragePoint[0].EventRecord.Remote=false&\
        //      RecordStoragePoint[0].EventSnapShot.FTP=false&\
        //      RecordStoragePoint[0].EventSnapShot.Remote=false&\
        //      RecordStoragePoint[0].ManualRecord.FTP=false&\
        //      RecordStoragePoint[0].ManualRecord.Remote=false&\
        //      RecordStoragePoint[0].ManualSnapShot.FTP=false&\
        //      RecordStoragePoint[0].ManualSnapShot.Remote=false&\
        //      RecordStoragePoint[0].TimingRecord.FTP=false&\
        //      RecordStoragePoint[0].TimingSnapShot.FTP=false&\
        //      RecordStoragePoint[0].VideoDetectRecord.FTP=false&\
        //      RecordStoragePoint[0].VideoDetectSnapShot.FTP=false\
        //      "
        //     .parse()
        //     .unwrap(),
        // )
        // .await?;

        log::trace!("{:?}: disable automatic recording", self.host);
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             RecordMode[0].Mode=2\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: set image mode to NTSC (for IVS)", self.host);
        self._request_text_ok(
            "/cgi-bin/configManager.cgi?action=setConfig&VideoStandard=NTSC"
                .parse()
                .unwrap(),
        )
        .await?;
        if let Err(error) = self._wait_for_power_down_up().await {
            log::warn!(
                "{:?}: error while waiting for reboot: {:?}",
                self.host,
                error
            );
        }

        log::trace!("{:?}: set Main Stream MAX settings", self.host);
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             Encode[0].MainFormat[0].Video.Compression=H.265&\
             Encode[0].MainFormat[0].Video.BitRateControl=VBR&\
             Encode[0].MainFormat[0].Video.Quality=6&\
             Encode[0].MainFormat[0].Video.BitRate=8192&\
             Encode[0].MainFormat[0].Audio.Compression=AAC&\
             Encode[0].MainFormat[0].Audio.Frequency=48000\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: set Sub Stream 1 medium settings", self.host);
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             Encode[0].ExtraFormat[0].Video.Compression=H.265&\
             Encode[0].ExtraFormat[0].Video.BitRateControl=VBR&\
             Encode[0].ExtraFormat[0].Video.BitRate=1024&\
             Encode[0].ExtraFormat[0].Audio.Compression=AAC&\
             Encode[0].ExtraFormat[0].Audio.Frequency=16000&\
             Encode[0].ExtraFormat[0].AudioEnable=true\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: set Sub Stream 2 low settings", self.host);
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             Encode[0].ExtraFormat[1].Video.Compression=H.265&\
             Encode[0].ExtraFormat[1].Video.BitRateControl=VBR&\
             Encode[0].ExtraFormat[1].Video.BitRate=256&\
             Encode[0].ExtraFormat[1].VideoEnable=true\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: disable video watermark", self.host);
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             VideoWaterMark[0].Enable=false\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: set channel name overlay", self.host);
        if let Some(video_overlay) = &sane_defaults_config.video_overlay {
            self._request_text_ok(
                format!(
                    "\
                     /cgi-bin/configManager.cgi?action=setConfig&\
                     ChannelTitle[0].Name={}&\
                     VideoWidget[0].ChannelTitle.EncodeBlend=true\
                     ",
                    percent_encoding::utf8_percent_encode(
                        video_overlay,
                        percent_encoding::NON_ALPHANUMERIC
                    )
                )
                .parse()
                .unwrap(),
            )
            .await?;
        } else {
            self._request_text_ok(
                "\
                 /cgi-bin/configManager.cgi?action=setConfig&\
                 ChannelTitle[0].Name=&\
                 VideoWidget[0].ChannelTitle.EncodeBlend=false\
                 "
                .parse()
                .unwrap(),
            )
            .await?;
        }

        log::trace!("{:?}: set profile manager to Normal", self.host);
        self._request_text_ok(
            "/cgi-bin/configManager.cgi?action=setConfig&VideoInMode[0].Config[0]=2"
                .parse()
                .unwrap(),
        )
        .await?;

        log::trace!("{:?}: enable and configure motion detection", self.host);
        self._request_text_ok(
            "/cgi-bin/configManager.cgi?action=setConfig&MotionDetect[0].Enable=true"
                .parse()
                .unwrap(),
        )
        .await?;

        log::trace!(
            "{:?}: enable and configure video tampering detection",
            self.host
        );
        self._request_text_ok(
            "/cgi-bin/configManager.cgi?action=setConfig&BlindDetect[0].Enable=true"
                .parse()
                .unwrap(),
        )
        .await?;

        log::trace!(
            "{:?}: enable and configure video scene change detection",
            self.host
        );
        self._request_text_ok(
            "/cgi-bin/configManager.cgi?action=setConfig&MovedDetect[0].Enable=true"
                .parse()
                .unwrap(),
        )
        .await?;

        log::trace!("{:?}: enable and configure audio detection", self.host);
        self._request_text_ok(
            "\
             /cgi-bin/configManager.cgi?action=setConfig&\
             AudioDetect[0].AnomalyDetect=true&\
             AudioDetect[0].MutationDetect=true&\
             AudioDetect[0].Enable=true\
             "
            .parse()
            .unwrap(),
        )
        .await?;

        log::trace!("{:?}: set auto old files cleanup", self.host);
        self._request_text_ok(
            "/cgi-bin/configManager.cgi?action=setConfig&StorageGlobal.FileHoldTime=1"
                .parse()
                .unwrap(),
        )
        .await?;

        log::trace!("{:?}: reboot and wait for device to be ready", self.host);
        self.reboot_wait_for_ready().await?;

        log::info!("{:?}: configuration complete", self.host);

        Ok(())
    }
}
