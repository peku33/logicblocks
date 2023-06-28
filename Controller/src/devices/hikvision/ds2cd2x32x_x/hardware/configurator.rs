use super::api::{Api, BasicDeviceInfo};
use anyhow::{bail, ensure, Context, Error};
use std::{fmt, marker::PhantomData, time::Duration};
use xmltree::{Element, XMLNode};

#[derive(Clone, Copy, Debug)]
pub struct Capabilities {
    audio: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Percentage {
    value: u8,
}
impl Percentage {
    pub fn new(value: u8) -> Result<Self, Error> {
        ensure!(value <= 100, "value must be at most 100");
        Ok(Self { value })
    }

    pub fn value(&self) -> u8 {
        self.value
    }
}

pub trait CoordinateSystem: Copy + Clone + fmt::Debug {
    fn x_min() -> usize;
    fn x_max() -> usize;
    fn y_min() -> usize;
    fn y_max() -> usize;
}
#[derive(Clone, Copy, Debug)]
pub struct CoordinateSystem704x576 {}
impl CoordinateSystem for CoordinateSystem704x576 {
    fn x_min() -> usize {
        0
    }
    fn x_max() -> usize {
        704
    }
    fn y_min() -> usize {
        0
    }
    fn y_max() -> usize {
        576
    }
}
#[derive(Clone, Copy, Debug)]
pub struct CoordinateSystem1000x1000 {}
impl CoordinateSystem for CoordinateSystem1000x1000 {
    fn x_min() -> usize {
        0
    }
    fn x_max() -> usize {
        1000
    }
    fn y_min() -> usize {
        0
    }
    fn y_max() -> usize {
        1000
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Coordinate<CS: CoordinateSystem> {
    x: usize,
    y: usize,

    _p: PhantomData<CS>,
}
impl<CS: CoordinateSystem> Coordinate<CS> {
    pub fn new(
        x: usize,
        y: usize,
    ) -> Result<Self, Error> {
        ensure!(x >= CS::x_min(), "x must be at least {}", CS::x_min());
        ensure!(y >= CS::y_min(), "y must be at least {}", CS::y_min());

        ensure!(x <= CS::x_max(), "x must be at most {}", CS::x_max());
        ensure!(y <= CS::y_max(), "y must be at most {}", CS::y_max());

        Ok(Self {
            x,
            y,
            _p: PhantomData,
        })
    }

    pub fn bottom_left() -> Self {
        Self {
            x: CS::x_min(),
            y: CS::y_min(),
            _p: PhantomData,
        }
    }

    pub fn bottom_right() -> Self {
        Self {
            x: CS::x_max(),
            y: CS::y_min(),
            _p: PhantomData,
        }
    }

    pub fn top_right() -> Self {
        Self {
            x: CS::x_max(),
            y: CS::y_max(),
            _p: PhantomData,
        }
    }

    pub fn top_left() -> Self {
        Self {
            x: CS::x_min(),
            y: CS::y_max(),
            _p: PhantomData,
        }
    }
}

pub trait CoordinateList<CS: CoordinateSystem>: Copy + Clone + fmt::Debug {
    fn list_name() -> &'static str;
    fn element_name() -> &'static str;
    fn coordinates_list(&self) -> Box<[Coordinate<CS>]>;
}

#[derive(Clone, Copy, Debug)]
pub struct RegionSquare<CS: CoordinateSystem> {
    bottom_left: Coordinate<CS>,
    top_right: Coordinate<CS>,
}
impl<CS: CoordinateSystem> RegionSquare<CS> {
    pub fn new(
        bottom_left: Coordinate<CS>,
        top_right: Coordinate<CS>,
    ) -> Result<Self, Error> {
        ensure!(bottom_left.x < top_right.x, "inverted square coords");
        ensure!(bottom_left.y < top_right.y, "inverted square coords");

        Ok(Self {
            bottom_left,
            top_right,
        })
    }

    pub fn null() -> Self {
        Self {
            bottom_left: Coordinate::bottom_left(),
            top_right: Coordinate::bottom_left(),
        }
    }

    pub fn full() -> Self {
        Self {
            bottom_left: Coordinate::bottom_left(),
            top_right: Coordinate::top_right(),
        }
    }
}
impl<CS: CoordinateSystem> CoordinateList<CS> for RegionSquare<CS> {
    fn list_name() -> &'static str {
        "RegionCoordinatesList"
    }
    fn element_name() -> &'static str {
        "RegionCoordinates"
    }
    fn coordinates_list(&self) -> Box<[Coordinate<CS>]> {
        vec![
            Coordinate::new(self.bottom_left.x, self.bottom_left.y).unwrap(),
            Coordinate::new(self.top_right.x, self.bottom_left.y).unwrap(),
            Coordinate::new(self.top_right.x, self.top_right.y).unwrap(),
            Coordinate::new(self.bottom_left.x, self.top_right.y).unwrap(),
        ]
        .into_boxed_slice()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RegionField4<CS: CoordinateSystem> {
    pub corners: [Coordinate<CS>; 4],
}
impl<CS: CoordinateSystem> RegionField4<CS> {
    pub fn null() -> Self {
        Self {
            corners: [
                Coordinate::bottom_left(),
                Coordinate::bottom_left(),
                Coordinate::bottom_left(),
                Coordinate::bottom_left(),
            ],
        }
    }

    pub fn full() -> Self {
        Self {
            corners: [
                Coordinate::bottom_left(),
                Coordinate::bottom_right(),
                Coordinate::top_right(),
                Coordinate::top_left(),
            ],
        }
    }
}
impl<CS: CoordinateSystem> CoordinateList<CS> for RegionField4<CS> {
    fn list_name() -> &'static str {
        "RegionCoordinatesList"
    }
    fn element_name() -> &'static str {
        "RegionCoordinates"
    }
    fn coordinates_list(&self) -> Box<[Coordinate<CS>]> {
        self.corners.to_vec().into_boxed_slice()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line<CS: CoordinateSystem> {
    pub from: Coordinate<CS>,
    pub to: Coordinate<CS>,
}
impl<CS: CoordinateSystem> CoordinateList<CS> for Line<CS> {
    fn list_name() -> &'static str {
        "CoordinatesList"
    }
    fn element_name() -> &'static str {
        "Coordinates"
    }
    fn coordinates_list(&self) -> Box<[Coordinate<CS>]> {
        vec![self.from, self.to].into_boxed_slice()
    }
}

#[derive(Clone, Debug)]
pub struct PrivacyMask {
    regions: Box<[RegionSquare<CoordinateSystem704x576>]>,
}
impl PrivacyMask {
    pub const REGIONS_MAX: usize = 4;

    pub fn new(regions: Box<[RegionSquare<CoordinateSystem704x576>]>) -> Result<Self, Error> {
        ensure!(
            regions.len() <= Self::REGIONS_MAX,
            "at most {} regions allowed",
            Self::REGIONS_MAX
        );
        Ok(Self { regions })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MotionDetectionRegion {
    pub region: RegionSquare<CoordinateSystem1000x1000>,
    pub sensitivity: Percentage,
    pub object_size: Percentage,
}
#[derive(Clone, Debug)]
pub struct MotionDetection {
    regions: Box<[MotionDetectionRegion]>,
}
impl MotionDetection {
    pub const REGIONS_MAX: usize = 8;

    pub fn new(regions: Box<[MotionDetectionRegion]>) -> Result<Self, Error> {
        ensure!(
            regions.len() <= Self::REGIONS_MAX,
            "number of regions could be at most {}",
            Self::REGIONS_MAX
        );
        Ok(Self { regions })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FieldDetection {
    pub region: RegionField4<CoordinateSystem1000x1000>,
    pub sensitivity: Percentage,
    pub object_occupation: Percentage,
    pub time_threshold_s: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum LineDetectionDirection {
    Both,
    RightToLeft,
    LeftToRight,
}
#[derive(Clone, Copy, Debug)]
pub struct LineDetection {
    pub line: Line<CoordinateSystem1000x1000>,
    pub direction: LineDetectionDirection,
    pub sensitivity: Percentage,
}

#[derive(Clone, Debug)]
pub struct Configuration {
    pub device_name: String,
    pub device_id: u8,
    pub shared_user_password: String,
    pub video_upside_down: bool,
    pub overlay_text: Option<String>,
    pub privacy_mask: Option<PrivacyMask>,
    pub motion_detection: Option<MotionDetection>,
    pub field_detection: Option<FieldDetection>,
    pub line_detection: Option<LineDetection>,
}

#[derive(Debug)]
pub struct Configurator<'a> {
    api: &'a Api,
    basic_device_info: BasicDeviceInfo,
    capabilities: Capabilities,
}
impl<'a> Configurator<'a> {
    pub const SHARED_USER_LOGIN: &'static str = "logicblocks";

    async fn capabilities_fetch(api: &Api) -> Result<Capabilities, Error> {
        let device_capabilities = api
            .get_xml("/ISAPI/System/capabilities".parse().unwrap())
            .await
            .context("get_xml")?;

        let system_capabilities = device_capabilities.get_child("SysCap");

        let audio_capabilities = match system_capabilities {
            Some(system_capabilities) => system_capabilities.get_child("AudioCap"),
            None => None,
        };
        let audio = audio_capabilities.is_some();

        let capabilities = Capabilities { audio };
        Ok(capabilities)
    }

    pub async fn connect(api: &'a Api) -> Result<Configurator<'a>, Error> {
        let basic_device_info = api
            .validate_basic_device_info()
            .await
            .context("validate_basic_device_info")?;
        let capabilities = Self::capabilities_fetch(api)
            .await
            .context("capabilities")?;

        let self_ = Self {
            api,
            basic_device_info,
            capabilities,
        };
        Ok(self_)
    }

    pub fn basic_device_info(&self) -> &BasicDeviceInfo {
        &self.basic_device_info
    }
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    async fn healthcheck(&mut self) -> Result<(), Error> {
        self.api
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
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        bail!("device didn't go away in designated time");
    }
    async fn wait_for_power_up(&mut self) -> Result<(), Error> {
        for _ in 0..60 {
            if self.healthcheck().await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        // TODO: Return last failure
        bail!("device didn't go up in designated time");
    }
    pub async fn reboot(&mut self) -> Result<(), Error> {
        self.api
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

    pub async fn system_factory_reset(&mut self) -> Result<(), Error> {
        let mut reboot_required = false;

        reboot_required |= self
            .api
            .put_xml(
                "/ISAPI/System/factoryReset?mode=basic".parse().unwrap(),
                None,
            )
            .await
            .context("put_xml")?
            .reboot_required;

        if reboot_required {
            self.reboot_wait_for_ready()
                .await
                .context("reboot_wait_for_ready")?;
        }
        Ok(())
    }
    pub async fn system_device_id_name(
        &mut self,
        device_id: u8,
        device_name: String,
    ) -> Result<(), Error> {
        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/System/deviceInfo".parse().unwrap(),
                Some(element_build_children(
                    "DeviceInfo",
                    vec![
                        element_build_text("deviceName", device_name),
                        element_build_text("telecontrolID", device_id.to_string()),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn system_time_gmt(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/System/time".parse().unwrap(),
                Some(element_build_children(
                    "Time",
                    vec![
                        element_build_text("timeMode", "NTP"),
                        element_build_text("timeZone", "CST+0:00:00"),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn system_time_ntp(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .api
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
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn system_shared_user(
        &mut self,
        password: String,
    ) -> Result<(), Error> {
        // Check if user is already added
        let user_ids = self
            .api
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
            .collect::<Box<[_]>>();

        // If so - delete
        for user_id in user_ids.iter() {
            let reboot_required = self
                .api
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
            .api
            .post_xml(
                "/ISAPI/Security/users".parse().unwrap(),
                Some(element_build_children(
                    "User",
                    vec![
                        element_build_text("userName", Self::SHARED_USER_LOGIN),
                        element_build_text("password", password),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("post_xml")?;
        ensure!(!post_result.reboot_required, "reboot is not supported here");

        // Set user permissions
        let reboot_required = self
            .api
            .put_xml(
                format!("/ISAPI/Security/UserPermission/{}", post_result.id)
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "UserPermission",
                    vec![
                        element_build_text("userID", post_result.id.to_string()),
                        element_build_text("userType", "viewer"),
                        element_build_children(
                            "remotePermission",
                            vec![element_build_bool("preview", true)].into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    pub async fn network_upnp_sane(
        &mut self,
        device_name: String,
    ) -> Result<(), Error> {
        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/System/Network/UPnP".parse().unwrap(),
                Some(element_build_children(
                    "UPnP",
                    vec![
                        element_build_bool("enabled", true),
                        element_build_text("name", device_name),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn network_port_mapping_disable(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/System/Network/UPnP/ports".parse().unwrap(),
                Some(element_build_children(
                    "ports",
                    vec![
                        element_build_bool("enabled", false),
                        element_build_text("mapmode", "auto"),
                        element_build_children("portList", vec![].into_boxed_slice()),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn network_ezviz_disable(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/System/Network/EZVIZ".parse().unwrap(),
                Some(element_build_children(
                    "EZVIZ",
                    vec![element_build_bool("enabled", false)].into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    pub async fn video_main_quality(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .api
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
                            ]
                            .into_boxed_slice(),
                        ),
                        element_build_children(
                            "Audio",
                            vec![element_build_bool("enabled", self.capabilities.audio)]
                                .into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn video_sub_quality(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .api
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
                            ]
                            .into_boxed_slice(),
                        ),
                        element_build_children(
                            "Audio",
                            vec![element_build_bool("enabled", self.capabilities.audio)]
                                .into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn video_upside_down(
        &mut self,
        upside_down: bool,
    ) -> Result<(), Error> {
        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/Image/channels/1/imageFlip".parse().unwrap(),
                Some(element_build_children(
                    "ImageFlip",
                    vec![
                        element_build_bool("enabled", upside_down),
                        element_build_text("ImageFlipStyle", "UPDOWN"),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    pub async fn audio(&mut self) -> Result<(), Error> {
        if !self.capabilities.audio {
            return Ok(());
        }

        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/System/TwoWayAudio/channels/1".parse().unwrap(),
                Some(element_build_children(
                    "TwoWayAudioChannel",
                    vec![
                        element_build_text("id", "1"),
                        element_build_bool("enabled", true),
                        element_build_text("audioInputType", "MicIn"),
                        element_build_text("speakerVolume", "100"),
                        element_build_bool("noisereduce", true),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    pub async fn image_overlay_text(
        &mut self,
        name: Option<String>,
    ) -> Result<(), Error> {
        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1/overlays"
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "VideoOverlay",
                    vec![element_build_children(
                        "channelNameOverlay",
                        vec![
                            element_build_bool("enabled", name.is_some()),
                            element_build_text("positionX", "512"),
                            element_build_text("positionY", "64"),
                        ]
                        .into_boxed_slice(),
                    )]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1".parse().unwrap(),
                Some(element_build_children(
                    "VideoInputChannel",
                    vec![
                        element_build_text("id", "1"),
                        element_build_text("inputPort", "1"),
                        element_build_text("name", name.as_deref().unwrap_or("")),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn image_overlay_date(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1/overlays"
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "VideoOverlay",
                    vec![element_build_children(
                        "DateTimeOverlay",
                        vec![
                            element_build_bool("enabled", true),
                            element_build_text("positionX", "0"),
                            element_build_text("positionY", "544"),
                            element_build_text("dateStyle", "YYYY-MM-DD"),
                            element_build_text("timeStyle", "24hour"),
                            element_build_bool("displayWeek", false),
                        ]
                        .into_boxed_slice(),
                    )]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn image_privacy_mask_enable(
        &mut self,
        privacy_mask: PrivacyMask,
    ) -> Result<(), Error> {
        let mut reboot_required = false;

        reboot_required |= self
            .api
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1/privacyMask"
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "PrivacyMask",
                    vec![
                        element_build_bool("enabled", true),
                        element_build_children(
                            "PrivacyMaskRegionList",
                            privacy_mask
                                .regions
                                .iter()
                                .enumerate()
                                .map(|(id, region)| {
                                    element_build_children(
                                        "PrivacyMaskRegion",
                                        vec![
                                            element_build_text("id", (id + 1).to_string()),
                                            element_build_bool("enabled", true),
                                            serialize_coordinates_list(region),
                                        ]
                                        .into_boxed_slice(),
                                    )
                                })
                                .collect::<Box<[_]>>(),
                        ),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    pub async fn record_schedule_disable(&mut self) -> Result<(), Error> {
        let reboot_required = self
            .api
            .put_xml(
                "/ISAPI/ContentMgmt/record/tracks/101".parse().unwrap(),
                Some(element_build_children(
                    "Track",
                    vec![
                        element_build_text("id", "101"),
                        element_build_text("Channel", "101"),
                        element_build_bool("Enable", false),
                        element_build_bool("LoopEnable", true),
                    ]
                    .into_boxed_slice(),
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
        motion_detection: MotionDetection,
    ) -> Result<(), Error> {
        let mut reboot_required = false;

        reboot_required |= self
            .api
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1/motionDetectionExt"
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "MotionDetectionExt",
                    vec![
                        element_build_bool("enabled", true),
                        element_build_text("activeMode", "expert"),
                        element_build_children(
                            "MotionDetectionRegionList",
                            motion_detection
                                .regions
                                .iter()
                                .enumerate()
                                .map(|(id, region)| {
                                    element_build_children(
                                        "MotionDetectionRegion",
                                        vec![
                                            element_build_text("id", (id + 1).to_string()),
                                            element_build_bool("enabled", true),
                                            element_build_text(
                                                "sensitivityLevel",
                                                region.sensitivity.value().to_string(),
                                            ),
                                            element_build_text(
                                                "objectSize",
                                                region.object_size.value().to_string(),
                                            ),
                                            serialize_coordinates_list(&region.region),
                                        ]
                                        .into_boxed_slice(),
                                    )
                                })
                                .collect::<Box<[_]>>(),
                        ),
                    ]
                    .into_boxed_slice(),
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
            .api
            .put_xml(
                "/ISAPI/System/Video/inputs/channels/1/tamperDetection"
                    .parse()
                    .unwrap(),
                Some(element_build_children(
                    "TamperDetection",
                    vec![
                        element_build_bool("enabled", true),
                        element_build_children(
                            "TamperDetectionRegionList",
                            vec![element_build_children(
                                "TamperDetectionRegion",
                                vec![
                                    element_build_text("id", "1"),
                                    element_build_bool("enabled", true),
                                    element_build_text("sensitivityLevel", "100"),
                                    serialize_coordinates_list(&RegionSquare::<
                                        CoordinateSystem704x576,
                                    >::full(
                                    )),
                                ]
                                .into_boxed_slice(),
                            )]
                            .into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn detection_field_enable(
        &mut self,
        field_detection: FieldDetection,
    ) -> Result<(), Error> {
        let mut reboot_required = false;

        reboot_required |= self
            .api
            .put_xml(
                "/ISAPI/Smart/FieldDetection/1".parse().unwrap(),
                Some(element_build_children(
                    "FieldDetection",
                    vec![
                        element_build_text("id", "1"),
                        element_build_bool("enabled", true),
                        element_build_children(
                            "normalizedScreenSize",
                            vec![
                                element_build_text("normalizedScreenWidth", "1000"),
                                element_build_text("normalizedScreenHeight", "1000"),
                            ]
                            .into_boxed_slice(),
                        ),
                        element_build_children(
                            "FieldDetectionRegionList",
                            vec![element_build_children(
                                "FieldDetectionRegion",
                                vec![
                                    element_build_text("id", "1"),
                                    element_build_bool("enabled", true),
                                    element_build_text(
                                        "sensitivityLevel",
                                        field_detection.sensitivity.value().to_string(),
                                    ),
                                    element_build_text(
                                        "objectOccupation",
                                        field_detection.object_occupation.value().to_string(),
                                    ),
                                    element_build_text(
                                        "timeThreshold",
                                        field_detection.time_threshold_s.to_string(),
                                    ),
                                    serialize_coordinates_list(&field_detection.region),
                                ]
                                .into_boxed_slice(),
                            )]
                            .into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }
    pub async fn detection_line_enable(
        &mut self,
        line_detection: LineDetection,
    ) -> Result<(), Error> {
        let mut reboot_required = false;

        reboot_required |= self
            .api
            .put_xml(
                "/ISAPI/Smart/LineDetection/1".parse().unwrap(),
                Some(element_build_children(
                    "LineDetection",
                    vec![
                        element_build_text("id", "1"),
                        element_build_bool("enabled", true),
                        element_build_children(
                            "normalizedScreenSize",
                            vec![
                                element_build_text("normalizedScreenWidth", "1000"),
                                element_build_text("normalizedScreenHeight", "1000"),
                            ]
                            .into_boxed_slice(),
                        ),
                        element_build_children(
                            "LineItemList",
                            vec![element_build_children(
                                "LineItem",
                                vec![
                                    element_build_text("id", "1"),
                                    element_build_bool("enabled", true),
                                    element_build_text(
                                        "sensitivityLevel",
                                        line_detection.sensitivity.value().to_string(),
                                    ),
                                    element_build_text(
                                        "directionSensitivity",
                                        match line_detection.direction {
                                            LineDetectionDirection::Both => "any",
                                            LineDetectionDirection::LeftToRight => "left-right",
                                            LineDetectionDirection::RightToLeft => "right-left",
                                        },
                                    ),
                                    serialize_coordinates_list(&line_detection.line),
                                ]
                                .into_boxed_slice(),
                            )]
                            .into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                )),
            )
            .await
            .context("put_xml")?
            .reboot_required;
        ensure!(!reboot_required, "reboot is not supported here");

        Ok(())
    }

    pub async fn configure(
        &mut self,
        configuration: Configuration,
    ) -> Result<(), Error> {
        // TODO: Progress callback

        self.system_factory_reset()
            .await
            .context("system_factory_reset")?;

        self.system_device_id_name(configuration.device_id, configuration.device_name.clone())
            .await
            .context("system_device_id_name")?;

        self.system_time_gmt() // break
            .await
            .context("system_time_gmt")?;

        self.system_time_ntp() // break
            .await
            .context("system_time_ntp")?;

        self.system_shared_user(configuration.shared_user_password)
            .await
            .context("system_shared_user")?;

        self.network_upnp_sane(configuration.device_name)
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

        self.video_upside_down(configuration.video_upside_down)
            .await
            .context("video_upside_down")?;

        self.audio() // line break
            .await
            .context("audio")?;

        self.image_overlay_text(configuration.overlay_text)
            .await
            .context("image_overlay_text")?;

        self.image_overlay_date()
            .await
            .context("image_overlay_date")?;

        if let Some(privacy_mask) = configuration.privacy_mask {
            self.image_privacy_mask_enable(privacy_mask)
                .await
                .context("image_privacy_mask_enable")?;
        }

        self.record_schedule_disable()
            .await
            .context("record_schedule_disable")?;

        if let Some(motion_detection) = configuration.motion_detection {
            self.detection_motion_enable(motion_detection)
                .await
                .context("detection_motion_enable")?;
        }

        self.detection_tamper_enable()
            .await
            .context("detection_tamper_enable")?;

        if let Some(field_detection) = configuration.field_detection {
            self.detection_field_enable(field_detection)
                .await
                .context("detection_field_enable")?;
        }

        if let Some(line_detection) = configuration.line_detection {
            self.detection_line_enable(line_detection)
                .await
                .context("detection_line_enable")?;
        }

        Ok(())
    }
}

fn element_build_text(
    name: &str,
    text: impl ToString,
) -> Element {
    let mut element = Element::new(name);
    element.children.push(XMLNode::Text(text.to_string()));
    element
}
fn element_build_bool(
    name: &str,
    value: bool,
) -> Element {
    element_build_text(name, if value { "true" } else { "false" })
}
fn element_build_children(
    name: &str,
    children: Box<[Element]>,
) -> Element {
    let mut element = Element::new(name);
    element.children = children
        .into_vec()
        .into_iter()
        .map(XMLNode::Element)
        .collect::<Vec<_>>();
    element
}

fn serialize_coordinates_list<CS: CoordinateSystem, C: CoordinateList<CS>>(
    coordinates_list: &C
) -> Element {
    element_build_children(
        C::list_name(),
        coordinates_list
            .coordinates_list()
            .iter()
            .map(|coordinate| {
                element_build_children(
                    C::element_name(),
                    vec![
                        element_build_text("positionX", coordinate.x.to_string()),
                        element_build_text("positionY", coordinate.y.to_string()),
                    ]
                    .into_boxed_slice(),
                )
            })
            .collect::<Box<[_]>>(),
    )
}
