use super::api::{Api, BasicDeviceInfo, WebVersion};
use anyhow::{Context, Error, anyhow, bail, ensure};
use arrayvec::ArrayVec;
use maplit::hashmap;
use md5::{Digest, Md5};
use serde_json::json;
use std::{cmp::max, collections::HashMap, iter, time::Duration};

#[derive(Clone, Copy, Debug)]
pub struct Percentage {
    value: u8,
}
impl Percentage {
    pub fn new(value: u8) -> Result<Self, Error> {
        ensure!(value <= 100, "value must be at most 100");
        Ok(Self { value })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sensitivity {
    value: u8,
}
impl Sensitivity {
    pub fn new(value: u8) -> Result<Self, Error> {
        ensure!((1..6).contains(&value), "value must be between 1 and 6");
        Ok(Self { value })
    }
}

// coordinate system
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Coordinate {
    value: u16,
}
impl Coordinate {
    pub const VALUE_MIN: u16 = 0;
    pub const VALUE_MAX: u16 = 8191;

    pub fn new(value: u16) -> Result<Self, Error> {
        // ensure!(value >= Self::VALUE_MIN);
        ensure!(value <= Self::VALUE_MAX);

        Ok(Self { value })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Point {
    // 0 is left
    x: Coordinate,

    // 0 is top
    y: Coordinate,
}
impl Point {
    pub fn new(
        x: Coordinate,
        y: Coordinate,
    ) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RegionSquare {
    top_left: Point,
    bottom_right: Point,
}
impl RegionSquare {
    pub fn new(
        top_left: Point,
        bottom_right: Point,
    ) -> Result<Self, Error> {
        ensure!(top_left.x <= bottom_right.x);
        ensure!(top_left.y <= bottom_right.y);

        Ok(Self {
            top_left,
            bottom_right,
        })
    }

    pub fn as_coords(&self) -> [u16; 4] {
        [
            self.top_left.x.value,
            self.top_left.y.value,
            self.bottom_right.x.value,
            self.bottom_right.y.value,
        ]
    }
}

// overlays
#[derive(Clone, Copy, Debug)]
pub struct PrivacyMaskRegion {
    pub region_square: RegionSquare,
}

#[derive(Clone, Debug)]
pub struct PrivacyMask {
    pub regions: ArrayVec<PrivacyMaskRegion, { Self::REGIONS_MAX }>,
}
impl PrivacyMask {
    pub const REGIONS_MAX: usize = 4;

    pub fn single(region: PrivacyMaskRegion) -> Self {
        Self {
            regions: iter::once(region).collect::<ArrayVec<_, { Self::REGIONS_MAX }>>(),
        }
    }
    pub fn none() -> Self {
        Self {
            regions: iter::empty().collect::<ArrayVec<_, { Self::REGIONS_MAX }>>(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Grid22x18 {
    grid: [[bool; Self::COLUMNS]; Self::ROWS], // from top-left corner
}
impl Grid22x18 {
    pub const COLUMNS: usize = 22;
    pub const ROWS: usize = 18;

    pub fn new(grid: [[bool; Self::COLUMNS]; Self::ROWS]) -> Self {
        Self { grid }
    }

    pub fn full() -> Self {
        Self::new([[true; Self::COLUMNS]; Self::ROWS])
    }
    pub fn empty() -> Self {
        Self::new([[false; Self::COLUMNS]; Self::ROWS])
    }

    fn as_rows_ltr(&self) -> [u32; Self::ROWS] {
        self.grid
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .fold(0, |mut accumulator, (index, enabled)| {
                        accumulator |= (*enabled as u32) << index;
                        accumulator
                    })
            })
            .collect::<ArrayVec<_, { Self::ROWS }>>()
            .into_inner()
            .unwrap()
    }
    fn as_rows_rtl(&self) -> [u32; Self::ROWS] {
        self.grid
            .iter()
            .map(|row| {
                row.iter()
                    .rev()
                    .enumerate()
                    .fold(0, |mut accumulator, (index, enabled)| {
                        accumulator |= (*enabled as u32) << index;
                        accumulator
                    })
            })
            .collect::<ArrayVec<_, { Self::ROWS }>>()
            .into_inner()
            .unwrap()
    }

    fn as_region(&self) -> RegionSquare {
        let grid_x_min = self
            .grid
            .iter()
            .filter_map(|row| {
                row.iter()
                    .enumerate()
                    .filter_map(|(index, cell)| if *cell { Some(index) } else { None })
                    .next()
            })
            .min()
            .unwrap_or(0);

        let grid_x_max = self
            .grid
            .iter()
            .filter_map(|row| {
                row.iter()
                    .enumerate()
                    .rev()
                    .filter_map(|(index, cell)| if *cell { Some(index) } else { None })
                    .next()
            })
            .max()
            .map(|x| x + 1)
            .unwrap_or(0);

        let grid_y_min = self
            .grid
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                if row.iter().any(|cell| *cell) {
                    Some(index)
                } else {
                    None
                }
            })
            .min()
            .unwrap_or(0);

        let grid_y_max = self
            .grid
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                if row.iter().any(|cell| *cell) {
                    Some(index)
                } else {
                    None
                }
            })
            .max()
            .map(|y| y + 1)
            .unwrap_or(0);

        RegionSquare::new(
            Point::new(
                Coordinate::new(
                    (1.0 * (grid_x_min as f64) / (Self::COLUMNS as f64)
                        * (Coordinate::VALUE_MAX as f64))
                        .floor() as u16,
                )
                .unwrap(),
                Coordinate::new(
                    (1.0 * (grid_y_min as f64) / (Self::ROWS as f64)
                        * (Coordinate::VALUE_MAX as f64))
                        .floor() as u16,
                )
                .unwrap(),
            ),
            Point::new(
                Coordinate::new(
                    (1.0 * (grid_x_max as f64) / (Self::COLUMNS as f64)
                        * (Coordinate::VALUE_MAX as f64))
                        .floor() as u16,
                )
                .unwrap(),
                Coordinate::new(
                    (1.0 * (grid_y_max as f64) / (Self::ROWS as f64)
                        * (Coordinate::VALUE_MAX as f64))
                        .floor() as u16,
                )
                .unwrap(),
            ),
        )
        .unwrap()
    }
}
#[cfg(test)]
mod tests_grid22x18 {
    use super::{Coordinate, Grid22x18, Point, RegionSquare};

    #[test]
    fn empty() {
        let grid = Grid22x18::empty();
        assert_eq!(grid.as_rows_ltr(), [0; Grid22x18::ROWS]);
        assert_eq!(grid.as_rows_rtl(), [0; Grid22x18::ROWS]);
        assert_eq!(
            grid.as_region(),
            RegionSquare::new(
                Point::new(Coordinate::new(0).unwrap(), Coordinate::new(0).unwrap()),
                Point::new(Coordinate::new(0).unwrap(), Coordinate::new(0).unwrap())
            )
            .unwrap()
        );
    }

    #[test]
    fn full() {
        let grid = Grid22x18::full();
        assert_eq!(grid.as_rows_ltr(), [4194303; 18]);
        assert_eq!(grid.as_rows_rtl(), [4194303; 18]);
        assert_eq!(
            grid.as_region(),
            RegionSquare::new(
                Point::new(Coordinate::new(0).unwrap(), Coordinate::new(0).unwrap()),
                Point::new(
                    Coordinate::new(8191).unwrap(),
                    Coordinate::new(8191).unwrap()
                )
            )
            .unwrap()
        );
    }

    #[test]
    fn top_left() {
        let mut grid = [[false; Grid22x18::COLUMNS]; Grid22x18::ROWS];
        grid[0][0] = true;

        let grid = Grid22x18::new(grid);
        assert_eq!(
            grid.as_rows_ltr(),
            [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            grid.as_rows_rtl(),
            [2097152, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            grid.as_region(),
            RegionSquare::new(
                Point::new(Coordinate::new(0).unwrap(), Coordinate::new(0).unwrap()),
                Point::new(Coordinate::new(372).unwrap(), Coordinate::new(455).unwrap())
            )
            .unwrap()
        );
    }

    #[test]
    fn random_1() {
        let mut grid = [[false; Grid22x18::COLUMNS]; Grid22x18::ROWS];

        grid[0][0] = true;
        grid[0][1] = true;

        grid[0][21] = true;
        grid[1][21] = true;
        grid[2][21] = true;

        grid[17][21] = true;
        grid[17][20] = true;
        grid[17][19] = true;
        grid[17][18] = true;

        grid[17][0] = true;
        grid[16][0] = true;
        grid[15][0] = true;
        grid[14][0] = true;
        grid[13][0] = true;

        let grid = Grid22x18::new(grid);
        assert_eq!(
            grid.as_rows_ltr(),
            [
                2097155, 2097152, 2097152, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 3932161
            ]
        );
        assert_eq!(
            grid.as_rows_rtl(),
            [
                3145729, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2097152, 2097152, 2097152, 2097152,
                2097167
            ]
        );
        assert_eq!(
            grid.as_region(),
            RegionSquare::new(
                Point::new(Coordinate::new(0).unwrap(), Coordinate::new(0).unwrap()),
                Point::new(
                    Coordinate::new(8191).unwrap(),
                    Coordinate::new(8191).unwrap()
                )
            )
            .unwrap()
        );
    }
    #[test]
    fn random_2() {
        let mut grid = [[false; Grid22x18::COLUMNS]; Grid22x18::ROWS];
        grid[2][1] = true;
        grid[4][21 - 3] = true;
        grid[17 - 5][21 - 6] = true;
        grid[17 - 7][8] = true;

        let grid = Grid22x18::new(grid);
        assert_eq!(
            grid.as_rows_ltr(),
            [
                0, 0, 2, 0, 262144, 0, 0, 0, 0, 0, 256, 0, 32768, 0, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            grid.as_rows_rtl(),
            [
                0, 0, 1048576, 0, 8, 0, 0, 0, 0, 0, 8192, 0, 64, 0, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            grid.as_region(),
            RegionSquare::new(
                Point::new(Coordinate::new(372).unwrap(), Coordinate::new(910).unwrap()),
                Point::new(
                    Coordinate::new(7074).unwrap(),
                    Coordinate::new(5915).unwrap()
                )
            )
            .unwrap()
        );
    }
}

#[derive(Clone, Debug)]
pub struct MotionDetectionRegion {
    pub name: String,
    pub grid: Grid22x18,
    pub sensitivity: Percentage,
    pub threshold: Percentage,
}

#[derive(Clone, Debug)]
pub struct MotionDetection {
    pub regions: ArrayVec<MotionDetectionRegion, { Self::REGIONS_MAX }>,
}
impl MotionDetection {
    pub const REGIONS_MAX: usize = 4;

    pub fn single(region: MotionDetectionRegion) -> Self {
        Self {
            regions: iter::once(region).collect::<ArrayVec<_, { Self::REGIONS_MAX }>>(),
        }
    }
    pub fn none() -> Self {
        Self {
            regions: iter::empty().collect::<ArrayVec<_, { Self::REGIONS_MAX }>>(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SmartMotionDetectionSensitivity {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug)]
pub struct SmartMotionDetection {
    pub human: bool,
    pub vehicle: bool,
    pub sensitivity: SmartMotionDetectionSensitivity,
}

#[derive(Clone, Copy, Debug)]
pub struct SceneMovedDetection {
    pub sensitivity: Sensitivity,
}

#[derive(Clone, Copy, Debug)]
pub struct AudioMutationDetection {
    pub sensitivity: Percentage,
}

// configuration
#[derive(Clone, Debug)]
pub struct Configuration {
    pub device_id: u8,
    pub device_name: String,
    pub shared_user_password: String,
    pub video_upside_down: bool,
    pub channel_title: Option<String>,
    pub privacy_mask: Option<PrivacyMask>,
    pub motion_detection: Option<MotionDetection>,
    pub smart_motion_detection: Option<SmartMotionDetection>,
    pub scene_moved_detection: Option<SceneMovedDetection>,
    pub audio_mutation_detection: Option<AudioMutationDetection>,
}

#[derive(Debug)]
pub struct Configurator<'a> {
    api: &'a Api,
    basic_device_info: BasicDeviceInfo,
}
impl<'a> Configurator<'a> {
    pub const SHARED_USER_LOGIN: &'static str = "logicblocks";

    pub async fn connect(api: &'a Api) -> Result<Configurator<'a>, Error> {
        let basic_device_info = api
            .validate_basic_device_info()
            .await
            .context("validate_basic_device_info")?;

        let self_ = Self {
            api,
            basic_device_info,
        };
        Ok(self_)
    }

    pub fn basic_device_info(&self) -> &BasicDeviceInfo {
        &self.basic_device_info
    }

    async fn healthcheck(&mut self) -> Result<(), Error> {
        self.api
            .validate_basic_device_info()
            .await
            .context("basic_device_info")?;
        Ok(())
    }

    async fn config_get(
        &mut self,
        name: &str,
    ) -> Result<serde_json::Value, Error> {
        let params = self
            .api
            .rpc2_call_params(
                "configManager.getConfig",
                json!({
                    "name": name,
                }),
            )
            .await
            .context("rpc2_call_params getConfig")?;

        let table = params
            .get("table")
            .ok_or_else(|| anyhow!("missing table"))?
            .clone();

        Ok(table)
    }
    async fn config_set(
        &mut self,
        name: &str,
        table: serde_json::Value,
    ) -> Result<(), Error> {
        let result = self
            .api
            .rpc2_call_params(
                "configManager.setConfig",
                json!({
                    "name": name,
                    "table": table,
                    "options": [],
                }),
            )
            .await
            .context("rpc2_call_params")?;

        let options = result
            .get("options")
            .ok_or_else(|| anyhow!("missing options"))?;

        if options == &json!(["NeedReboot"]) {
            log::trace!("device requested reboot at {name}");
            self.wait_for_power_down_up()
                .await
                .context("wait_for_power_down_up")?;
        }

        Ok(())
    }

    async fn config_patch_with<E>(
        &mut self,
        name: &str,
        executor: E,
    ) -> Result<(), Error>
    where
        E: FnOnce(&mut serde_json::Value) -> Result<(), Error>,
    {
        let mut table = self.config_get(name).await.context("config_get")?;

        executor(&mut table).context("executor")?;

        self.config_set(name, table).await.context("config_set")?;
        Ok(())
    }
    async fn config_patch_object_with<E>(
        &mut self,
        name: &str,
        executor: E,
    ) -> Result<(), Error>
    where
        E: FnOnce(&mut serde_json::Map<String, serde_json::Value>) -> Result<(), Error>,
    {
        self.config_patch_with(name, |config| -> Result<(), Error> {
            let config = config
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;

            executor(config).context("executor")?;

            Ok(())
        })
        .await
        .context("config_patch_with")?;

        Ok(())
    }
    async fn config_patch_array_object_with<E>(
        &mut self,
        name: &str,
        executor: E,
    ) -> Result<(), Error>
    where
        E: FnOnce(&mut serde_json::Map<String, serde_json::Value>) -> Result<(), Error>,
    {
        self.config_patch_with(name, |config| -> Result<(), Error> {
            let config = config
                .as_array_mut()
                .ok_or_else(|| anyhow!("expected array"))?;
            ensure!(config.len() == 1, "expected single item array");
            let config = config.get_mut(0).unwrap();

            let config = config
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;

            executor(config).context("executor")?;

            Ok(())
        })
        .await
        .context("config_patch_with")?;

        Ok(())
    }

    async fn config_patch_object(
        &mut self,
        name: &str,
        patch: HashMap<&str, serde_json::Value>,
    ) -> Result<(), Error> {
        self.config_patch_object_with(name, |config| -> Result<(), Error> {
            patch_object(config, patch).context("patch_object")?;

            Ok(())
        })
        .await
        .context("config_patch_object_with")?;

        Ok(())
    }
    async fn config_patch_array_object(
        &mut self,
        name: &str,
        patch: HashMap<&str, serde_json::Value>,
    ) -> Result<(), Error> {
        self.config_patch_array_object_with(name, |config| -> Result<(), Error> {
            patch_object(config, patch).context("patch_object")?;

            Ok(())
        })
        .await
        .context("config_patch_array_object_with")?;

        Ok(())
    }

    pub async fn dump(&mut self) -> Result<serde_json::Value, Error> {
        let config = self.config_get("All").await.context("config_get")?;

        Ok(config)
    }

    async fn wait_for_power_down(&mut self) -> Result<(), Error> {
        for _ in 0..60 {
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
    async fn wait_for_power_down_up(&mut self) -> Result<(), Error> {
        self.wait_for_power_down()
            .await
            .context("wait_for_power_down")?;

        self.wait_for_power_up()
            .await
            .context("wait_for_power_up")?;

        Ok(())
    }
    pub async fn reboot(&mut self) -> Result<(), Error> {
        self.api
            .rpc2_call_result("magicBox.reboot", serde_json::Value::Null)
            .await
            .context("rpc2_call_result")?;

        Ok(())
    }
    pub async fn reboot_wait_for_ready(&mut self) -> Result<(), Error> {
        self.reboot().await.context("reboot")?;
        self.wait_for_power_down_up()
            .await
            .context("wait_for_power_down_up")?;

        Ok(())
    }

    pub async fn system_factory_reset(&mut self) -> Result<(), Error> {
        loop {
            let mut again = false;

            let result = self
                .api
                .rpc2_call(
                    "configManager.restoreExcept",
                    json!({
                        "names": ["Network"],
                    }),
                    None,
                )
                .await
                .context("rpc2_call");

            if result.is_err() {
                again = true;
                log::warn!(
                    "error while resetting to factory settings, this is likely false positive (device bug)"
                );
            }

            // system restart MAY require reboot
            let rebooted = self.wait_for_power_down().await.is_ok();
            if rebooted {
                self.wait_for_power_up()
                    .await
                    .context("wait_for_power_up")?;
            } else {
                self.reboot_wait_for_ready()
                    .await
                    .context("reboot_wait_for_ready")?
            }

            if !again {
                break;
            }
        }

        Ok(())
    }
    pub async fn system_firmware_upgrade(&mut self) -> Result<(), Error> {
        let result = self
            .api
            .rpc2_call_params(
                "CloudUpgrader.check",
                json!({
                    "way": 0,
                }),
            )
            .await
            .context("rpc2_call_params check")?;

        let info = result
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("info")
            .ok_or_else(|| anyhow!("missing info"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?;

        let build = info
            .get("NewVersion")
            .ok_or_else(|| anyhow!("missing NewVersion"))?
            .as_str()
            .ok_or_else(|| anyhow!("expected string"))?;

        if build.is_empty() {
            // we are on latest available version
            log::trace!("no firmware upgrade available");
            return Ok(());
        }

        if !Api::update_build_supported(build) {
            log::warn!("firmware upgrade available, but {build} is not supported");
            // we continue with what we have
            return Ok(());
        }

        log::trace!("performing system upgrade to version {build}");
        self.api
            .rpc2_call_result(
                "CloudUpgrader.execute",
                json!({
                    "NewVersion": build,
                    "way": 0,
                }),
            )
            .await
            .context("rpc2_call_result execute")?;

        loop {
            let result = self
                .api
                .rpc2_call_params("CloudUpgrader.getState", serde_json::Value::Null)
                .await
                .context("rpc2_call_params getState")?;

            let state = result
                .as_object()
                .ok_or_else(|| anyhow!("expected object"))?
                .get("State")
                .ok_or_else(|| anyhow!("missing State"))?
                .as_str()
                .ok_or_else(|| anyhow!("expected string"))?;

            match state {
                "Preparing" | "Downloading" | "Upgrading" => {}
                "Succeeded" => break,
                _ => bail!("unknown update state: {state}"),
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        self.wait_for_power_down_up()
            .await
            .context("wait_for_power_down_up")?;

        Ok(())
    }

    pub async fn system_shared_user(
        &mut self,
        password: String,
    ) -> Result<(), Error> {
        // check existing users
        let user_infos = self
            .api
            .rpc2_call_params("userManager.getUserInfoAll", serde_json::Value::Null)
            .await
            .context("rpc2_call_params get user info")?;

        let user_infos = user_infos
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("users")
            .ok_or_else(|| anyhow!("missing users"))?
            .as_array()
            .ok_or_else(|| anyhow!("expected array"))?;

        let mut user_id_max: u64 = 1;
        let mut shared_user_exists = false;
        for user_info in user_infos {
            let user_info = user_info
                .as_object()
                .ok_or_else(|| anyhow!("expected object"))?;

            let user_id = user_info
                .get("Id")
                .ok_or_else(|| anyhow!("missing id"))?
                .as_u64()
                .ok_or_else(|| anyhow!("expected number"))?;
            user_id_max = max(user_id_max, user_id);

            let user_name = user_info
                .get("Name")
                .ok_or_else(|| anyhow!("missing name"))?
                .as_str()
                .ok_or_else(|| anyhow!("expected string"))?;
            if user_name == Self::SHARED_USER_LOGIN {
                shared_user_exists = true;
            }
        }

        // delete share user if exists
        if shared_user_exists {
            self.api
                .rpc2_call_result(
                    "userManager.deleteUser",
                    json!({
                        "name": Self::SHARED_USER_LOGIN,
                    }),
                )
                .await
                .context("rpc2_call_result delete user")?;
        } else {
            user_id_max += 1;
        }

        // create new user
        let realm = self
            .api
            .rpc2_session_peek_realm()
            .await
            .context("rpc2_session_peek_realm")?
            .ok_or_else(|| anyhow!("missing realm on api?"))?;

        let realm_phase = {
            let mut d = Md5::new();
            d.update(Self::SHARED_USER_LOGIN);
            d.update(":");
            d.update(realm);
            d.update(":");
            d.update(&password);
            let h = d.finalize();
            h
        };
        let realm_phase = hex::encode_upper(realm_phase);

        self.api
            .rpc2_call_result(
                "userManager.addUser",
                json!({
                    "user": {
                        "Id": user_id_max,
                        "Name": Self::SHARED_USER_LOGIN,
                        "Password": realm_phase,
                        "Type": "",
                        "ModifiedTime": "",
                        "Memo": "logicblocks system account",
                        "Group": "user",
                        "AuthorityList": ["Monitor_01"],
                        "Reserved": false,
                        "Sharable": true,
                    },
                }),
            )
            .await
            .context("rpc2_call_result add user")?;

        Ok(())
    }
    pub async fn system_arp_ip_setting_disable(&mut self) -> Result<(), Error> {
        // wtf which one is correct, both works

        // this one is listed in "All" configuration
        if self.basic_device_info.web_version
            == (WebVersion {
                major: 3,
                minor: 2,
                revision: 1,
                build: 582554,
            })
        {
            self.config_patch_object(
                "ARP: Ping",
                hashmap! {
                    "SettingIP" => json!(false),
                },
            )
            .await
            .context("config_patch_object colon")?;
        }

        // this one is set by GUI
        self.config_patch_object(
            "ARP&Ping",
            hashmap! {
                "SettingIP" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_device_discovery_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "DeviceDiscovery",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object and")?;

        Ok(())
    }
    pub async fn system_ipv6_enable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "IPv6",
            hashmap! {
                "Enable" => json!(true),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_multicast_disable(&mut self) -> Result<(), Error> {
        self.config_patch_with("Multicast", |config| {
            *config
                .pointer_mut("/DHII/0/Enable")
                .ok_or_else(|| anyhow!("missing item"))? = json!(false);

            *config
                .pointer_mut("/RTP/0/Enable")
                .ok_or_else(|| anyhow!("missing item"))? = json!(false);

            *config
                .pointer_mut("/RTP/1/Enable")
                .ok_or_else(|| anyhow!("missing item"))? = json!(false);

            Ok(())
        })
        .await
        .context("config_patch_with")?;

        Ok(())
    }

    pub async fn system_time_ntp(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "NTP",
            hashmap! {
                "Enable" => json!(true),
                "Address" => json!("pool.ntp.org"),
                "Port" => json!(123),
                "TimeZone" => json!(0),
                "UpdatePeriod" => json!(10),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }

    pub async fn system_snmp_disable(&mut self) -> Result<(), Error> {
        // TODO: check if this is covered by something like "get caps"
        if self.basic_device_info.web_version
            >= (WebVersion {
                major: 3,
                minor: 2,
                revision: 1,
                build: 1850781,
            })
        {
            return Ok(());
        }

        self.config_patch_object(
            "SNMP",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_upnp_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "UPnP",
            hashmap! {
                "Enable" => json!(false),
                "StartDeviceDiscover" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_easy4ip_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "T2UServer",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_bonjour_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "Bonjour",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_onvif_disable(&mut self) -> Result<(), Error> {
        // TODO: check if this is covered by something like "get caps"
        if self.basic_device_info.web_version
            < (WebVersion {
                major: 3,
                minor: 2,
                revision: 1,
                build: 582554,
            })
        {
            return Ok(());
        }

        self.config_patch_object(
            "VSP_ONVIF",
            hashmap! {
                "ServiceStart" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_genetec_disable(&mut self) -> Result<(), Error> {
        // TODO: check if this is covered by something like "get caps"
        if self.basic_device_info.web_version
            < (WebVersion {
                major: 3,
                minor: 2,
                revision: 1,
                build: 582554,
            })
        {
            return Ok(());
        }

        self.config_patch_object(
            "VSP_GENETEC",
            hashmap! {
                "ServiceStart" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_lechange_pro_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "VSP_PaaS",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_mobile_phone_platform_disable(&mut self) -> Result<(), Error> {
        // TODO: check if this is covered by something like "get caps"
        if self.basic_device_info.web_version
            < (WebVersion {
                major: 3,
                minor: 2,
                revision: 1,
                build: 582554,
            })
        {
            return Ok(());
        }

        self.config_patch_object(
            "MobilePhoneApplication",
            hashmap! {
                "PushNotificationEnable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_email_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "Email",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }

    pub async fn system_hostname_set(
        &mut self,
        hostname: &str,
    ) -> Result<(), Error> {
        self.config_patch_object(
            "Network",
            hashmap! {
                "Domain" => json!("logicblocks"),
                "Hostname" => json!(hostname),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_device_id_name_set(
        &mut self,
        device_id: u8,
        device_name: &str,
    ) -> Result<(), Error> {
        self.config_patch_object(
            "General",
            hashmap! {
                "LocalNo" => json!(device_id),
                "MachineName" => json!(device_name),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_old_files_delete_enable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "StorageGlobal",
            hashmap! {
                "FileHoldTime" => json!(7),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn system_storage_disable(&mut self) -> Result<(), Error> {
        self.config_patch_array_object_with("RecordStoragePoint", |config| -> Result<(), Error> {
            config
                .values_mut()
                .try_for_each(|config| -> Result<(), Error> {
                    let config = config
                        .as_object_mut()
                        .ok_or_else(|| anyhow!("expected object"))?;

                    // at least one element must be set to true, otherwise detections wont work
                    patch_object(
                        config,
                        hashmap! {
                            "AutoSync" => json!(false),
                            "Custom" => json!(true),
                            "FTP" => json!(false),
                            "Local" => json!(false),
                            "LocalForEmergency" => json!(false),
                            "Redundant" => json!(false),
                            "Remote" => json!(false),
                        },
                    )
                    .context("patch_object")?;

                    Ok(())
                })?;

            Ok(())
        })
        .await
        .context("config_patch_array_object_with")?;

        Ok(())
    }
    pub async fn system_record_disable(&mut self) -> Result<(), Error> {
        self.config_patch_array_object(
            "RecordMode",
            hashmap! {
                "Mode" => json!(2),
            },
        )
        .await
        .context("config_patch_array_object")?;

        Ok(())
    }
    pub async fn system_ntsc_set(&mut self) -> Result<(), Error> {
        // required for IVS to work
        let mut changed = false;

        let changed_ref = &mut changed;
        self.config_patch_with("VideoStandard", |config| {
            let config_new = json!("NTSC");
            if *config != config_new {
                *config = config_new;
                *changed_ref = true;
            }
            Ok(())
        })
        .await
        .context("config_patch_with")?;

        // change MAY require reboot
        if changed {
            let _ = self.wait_for_power_down().await;
            self.wait_for_power_up()
                .await
                .context("wait_for_power_up")?;
        }

        Ok(())
    }

    pub async fn video_ai_codec_disable(&mut self) -> Result<(), Error> {
        let encode_capabilities = self
            .api
            .rpc2_call_params("encode.getCaps", serde_json::Value::Null)
            .await
            .context("rpc2_call_params")?;

        if encode_capabilities.pointer("/caps/VideoEncodeDevices/0/SupportAICoding/0/AICoding")
            != Some(&json!(true))
        {
            return Ok(());
        }

        self.config_set(
            "AICoding",
            json!([{
                "Enable": false
            }]),
        )
        .await
        .context("config_set")?;

        Ok(())
    }
    pub async fn video_quality_configure(&mut self) -> Result<(), Error> {
        let encode_capabilities = self
            .api
            .rpc2_call_params("encode.getCaps", serde_json::Value::Null)
            .await
            .context("rpc2_call_params")?;

        let extra_streams_count = encode_capabilities
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("caps")
            .ok_or_else(|| anyhow!("missing MaxWidth"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("MaxExtraStream")
            .ok_or_else(|| anyhow!("missing MaxExtraStream"))?
            .as_u64()
            .ok_or_else(|| anyhow!("expected number"))? as usize;
        ensure!((1..=2).contains(&extra_streams_count));

        let video_input_capabilities = self
            .api
            .rpc2_call_params(
                "devVideoInput.getCaps",
                json!({
                    "channel": 0_usize,
                }),
            )
            .await
            .context("rpc2_call_params")?;
        let video_input_capabilities = video_input_capabilities
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("caps")
            .ok_or_else(|| anyhow!("missing caps"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?;

        let width = video_input_capabilities
            .get("MaxWidth")
            .ok_or_else(|| anyhow!("missing MaxWidth"))?
            .as_u64()
            .ok_or_else(|| anyhow!("expected number"))? as usize;

        let height = video_input_capabilities
            .get("MaxHeight")
            .ok_or_else(|| anyhow!("missing MaxHeight"))?
            .as_u64()
            .ok_or_else(|| anyhow!("expected number"))? as usize;

        fn apply_main_format(
            config: &mut serde_json::Value,
            width: usize,
            height: usize,
        ) -> Result<(), Error> {
            let config = config
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;
            patch_object(
                config,
                hashmap! {
                    "AudioEnable" => json!(true),
                    "VideoEnable" => json!(true),
                },
            )
            .context("patch_object config")?;

            let audio = config
                .get_mut("Audio")
                .ok_or_else(|| anyhow!("missing Audio"))?
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;
            patch_object(
                audio,
                hashmap! {
                    "Bitrate" => json!(64),
                    "Compression" => json!("AAC"),
                    "Depth" => json!(16),
                    "Frequency" => json!(32000),
                },
            )
            .context("patch_object audio")?;

            let video = config
                .get_mut("Video")
                .ok_or_else(|| anyhow!("missing Video"))?
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;
            patch_object(
                video,
                hashmap! {
                    "Compression" => json!("H.265"),
                    "Width" => json!(width),
                    "Height" => json!(height),
                    "CustomResolutionName" => json!(format!("{}x{}", width, height)),
                    "BitRateControl" => json!("VBR"),
                    "BitRate" => json!(8192),
                    "Quality" => json!(6),
                    "FPS" => json!(20),
                    "GOP" => json!(40),
                    "Profile" => json!("Main"),
                },
            )
            .context("patch_object video")?;

            Ok(())
        }
        fn apply_sub1_format(config: &mut serde_json::Value) -> Result<(), Error> {
            let config = config
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;
            patch_object(
                config,
                hashmap! {
                    "AudioEnable" => json!(true),
                    "VideoEnable" => json!(true),
                },
            )
            .context("patch_object config")?;

            let audio = config
                .get_mut("Audio")
                .ok_or_else(|| anyhow!("missing Audio"))?
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;
            patch_object(
                audio,
                hashmap! {
                    "Bitrate" => json!(32),
                    "Compression" => json!("AAC"),
                    "Depth" => json!(16),
                    "Frequency" => json!(8000),
                },
            )
            .context("patch_object audio")?;

            let video = config
                .get_mut("Video")
                .ok_or_else(|| anyhow!("missing Video"))?
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;
            patch_object(
                video,
                hashmap! {
                    "Compression" => json!("H.265"),
                    "Width" => json!(352),
                    "Height" => json!(240),
                    "CustomResolutionName" => json!("CIF"),
                    "BitRateControl" => json!("VBR"),
                    "BitRate" => json!(128),
                    "Quality" => json!(2),
                    "FPS" => json!(5),
                    "GOP" => json!(40),
                    "Profile" => json!("Main"),
                },
            )
            .context("patch_object video")?;

            Ok(())
        }
        fn apply_sub2_format(config: &mut serde_json::Value) -> Result<(), Error> {
            let config = config
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;
            patch_object(
                config,
                hashmap! {
                    "AudioEnable" => json!(true),
                    "VideoEnable" => json!(true),
                },
            )
            .context("patch_object config")?;

            let audio = config
                .get_mut("Audio")
                .ok_or_else(|| anyhow!("missing Audio"))?
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;
            patch_object(
                audio,
                hashmap! {
                    "Bitrate" => json!(64),
                    "Compression" => json!("AAC"),
                    "Depth" => json!(16),
                    "Frequency" => json!(16000),
                },
            )
            .context("patch_object audio")?;

            let video = config
                .get_mut("Video")
                .ok_or_else(|| anyhow!("missing Video"))?
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;
            patch_object(
                video,
                hashmap! {
                    "Compression" => json!("H.265"),
                    "Width" => json!(704),
                    "Height" => json!(480),
                    "CustomResolutionName" => json!("D1"),
                    "BitRateControl" => json!("VBR"),
                    "BitRate" => json!(512),
                    "Quality" => json!(4),
                    "FPS" => json!(10),
                    "GOP" => json!(40),
                    "Profile" => json!("Main"),
                },
            )
            .context("patch_object video")?;

            Ok(())
        }

        self.config_patch_array_object_with("Encode", |config| {
            let main_format = config
                .get_mut("MainFormat")
                .ok_or_else(|| anyhow!("missing MainFormat"))?
                .as_array_mut()
                .ok_or_else(|| anyhow!("expected array"))?;
            ensure!(main_format.len() == 4);
            main_format
                .iter_mut()
                .try_for_each(|config| apply_main_format(config, width, height))?;

            let extra_format = config
                .get_mut("ExtraFormat")
                .ok_or_else(|| anyhow!("missing ExtraFormat"))?
                .as_array_mut()
                .ok_or_else(|| anyhow!("expected array"))?;
            ensure!(extra_format.len() == 3);

            if extra_streams_count >= 1 {
                let sub1_format = extra_format.get_mut(0).unwrap();
                apply_sub1_format(sub1_format).context("apply_sub1_format")?;
            }

            if extra_streams_count >= 2 {
                let sub2_format = extra_format.get_mut(1).unwrap();
                apply_sub2_format(sub2_format).context("apply_sub2_format")?;
            }

            // sub3 format is not used?

            // TODO: Snap Format

            Ok(())
        })
        .await
        .context("config_patch_array_object_with")?;

        Ok(())
    }
    pub async fn video_watermark_disable(&mut self) -> Result<(), Error> {
        self.config_patch_array_object(
            "VideoWaterMark",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("video_watermark_disable")?;

        Ok(())
    }
    pub async fn video_profile_normal_only(&mut self) -> Result<(), Error> {
        self.config_patch_array_object(
            "VideoInMode",
            hashmap! {
                "Config" => json!([2]),
                "Mode" => json!(0),
            },
        )
        .await
        .context("config_patch_array_object")?;

        Ok(())
    }
    pub async fn video_orientation_configure(
        &mut self,
        upside_down: bool,
    ) -> Result<(), Error> {
        self.config_set(
            "VideoImageControl",
            json!([{
                "Flip": upside_down,
                "Freeze": false,
                "Mirror": false,
                "Rotate90": 0,
                "Stable": 0
            }]),
        )
        .await
        .context("config_set")?;

        Ok(())
    }
    pub async fn video_channel_title_configure(
        &mut self,
        channel_title: Option<String>,
    ) -> Result<(), Error> {
        if let Some(channel_title) = &channel_title {
            self.config_patch_array_object(
                "ChannelTitle",
                hashmap! {
                    "Name" => json!(channel_title),
                },
            )
            .await
            .context("config_patch_object")?;
        }

        self.config_patch_with("VideoWidget", |config| {
            *config
                .pointer_mut("/0/ChannelTitle/EncodeBlend")
                .ok_or_else(|| anyhow!("missing EncodeBlend"))? = json!(channel_title.is_some());

            *config
                .pointer_mut("/0/ChannelTitle/PreviewBlend")
                .ok_or_else(|| anyhow!("missing EncodeBlend"))? = json!(channel_title.is_some());

            Ok(())
        })
        .await
        .context("config_patch_with")?;

        Ok(())
    }
    pub async fn video_privacy_mask_configure(
        &mut self,
        privacy_mask: Option<PrivacyMask>,
    ) -> Result<(), Error> {
        fn patch_item(
            config: &mut serde_json::Value,
            privacy_mask_region: Option<PrivacyMaskRegion>,
        ) -> Result<(), Error> {
            let config = config
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;

            if let Some(privacy_mask_region) = privacy_mask_region {
                patch_object(
                    config,
                    hashmap! {
                        "Rect" => json!(privacy_mask_region.region_square.as_coords()),
                        "EncodeBlend" => json!(true),
                        "PreviewBlend" => json!(true),
                    },
                )
                .context("patch_object")?;
            } else {
                patch_object(
                    config,
                    hashmap! {
                        "EncodeBlend" => json!(false),
                        "PreviewBlend" => json!(false),
                    },
                )
                .context("patch_object")?;
            }

            Ok(())
        }

        let privacy_mask = privacy_mask.unwrap_or_else(PrivacyMask::none);

        self.config_patch_array_object_with("VideoWidget", |config| {
            let covers = config
                .get_mut("Covers")
                .ok_or_else(|| anyhow!("missing Covers"))?
                .as_array_mut()
                .ok_or_else(|| anyhow!("expected array"))?;

            ensure!(covers.len() >= privacy_mask.regions.len());

            covers
                .iter_mut()
                .zip(
                    privacy_mask
                        .regions
                        .into_iter()
                        .map(Some)
                        .chain(iter::repeat(None)),
                )
                .try_for_each(|(config, region_square)| {
                    patch_item(config, region_square).context("patch_item")
                })?;

            Ok(())
        })
        .await
        .context("config_patch_array_object_with")?;

        Ok(())
    }

    async fn detection_capabilities_get(
        &mut self
    ) -> Result<serde_json::Map<String, serde_json::Value>, Error> {
        let (detection_capabilities_key, _) = self
            .api
            .rpc2_call(
                "devVideoDetect.factory.instance",
                json!({
                    "channel": 0_usize,
                }),
                None,
            )
            .await
            .context("rpc2_call")?;
        let detection_capabilities_key = detection_capabilities_key
            .ok_or_else(|| anyhow!("missing result"))?
            .as_u64()
            .ok_or_else(|| anyhow!("expected number"))?;

        let (result, detection_capabilities) = self
            .api
            .rpc2_call(
                "devVideoDetect.getCaps",
                serde_json::Value::Null,
                Some(serde_json::Value::Number(detection_capabilities_key.into())),
            )
            .await
            .context("rpc2_call_params")?;

        let result = result
            .ok_or_else(|| anyhow!("missing result"))?
            .as_bool()
            .ok_or_else(|| anyhow!("expected bool"))?;
        ensure!(result, "request failed with result = {}", result);

        let detection_capabilities = detection_capabilities
            .ok_or_else(|| anyhow!("missing params"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("caps")
            .ok_or_else(|| anyhow!("missing caps"))?
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .clone();

        Ok(detection_capabilities)
    }
    pub async fn detection_external_alarm_disable(&mut self) -> Result<(), Error> {
        // TODO: check if this is covered by something like "get caps"
        if self.basic_device_info.web_version
            >= (WebVersion {
                major: 3,
                minor: 2,
                revision: 1,
                build: 1053483,
            })
        {
            return Ok(());
        }

        self.config_patch_with("ExAlarm", |config| -> Result<(), Error> {
            let config = config
                .as_array_mut()
                .ok_or_else(|| anyhow!("expected array"))?;

            config
                .iter_mut()
                .try_for_each(|config| -> Result<(), Error> {
                    let config = config
                        .as_object_mut()
                        .ok_or_else(|| anyhow!("expected object"))?;

                    patch_object(
                        config,
                        hashmap! {
                            "Enable" => json!(false),
                        },
                    )
                    .context("patch_object")?;

                    Ok(())
                })?;

            Ok(())
        })
        .await
        .context("config_patch_with")?;

        Ok(())
    }
    pub async fn detection_login_failure_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "LoginFailureAlarm",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn detection_network_conflict_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "IPConflict",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn detection_network_disconnected_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "NetAbort",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn detection_power_fault_disable(&mut self) -> Result<(), Error> {
        self.config_patch_array_object(
            "PowerFault",
            hashmap! {
                "Enable" => json!(false),
                "EncodeBlend" => json!(false),
            },
        )
        .await
        .context("config_patch_array_object")?;

        Ok(())
    }
    pub async fn detection_storage_health_alarm_disable(&mut self) -> Result<(), Error> {
        self.config_patch_object(
            "StorageHealthAlarm",
            hashmap! {
                "Enable" => json!(false),
            },
        )
        .await
        .context("config_patch_object")?;

        Ok(())
    }
    pub async fn detection_motion_configure(
        &mut self,
        detection_capabilities: &serde_json::Map<String, serde_json::Value>,
        motion_detection: Option<MotionDetection>,
    ) -> Result<(), Error> {
        let supported = detection_capabilities
            .get("SupportMotion")
            .ok_or_else(|| anyhow!("missing SupportMotion"))?
            .as_bool()
            .ok_or_else(|| anyhow!("expected bool"))?;
        if !supported {
            if motion_detection.is_some() {
                log::warn!("motion detection is not supported, skipping");
            }
            return Ok(());
        }

        self.config_patch_array_object_with("MotionDetect", |config| -> Result<(), Error> {
            patch_nested_event_handler(config).context("patch_nested_event_handler")?;

            patch_object(
                config,
                hashmap! {
                    "Enable" => json!(motion_detection.is_some())
                },
            )
            .context("patch_object")?;

            if let Some(motion_detection) = motion_detection {
                // detect version
                let mut detect_version_1 = false;
                let mut detect_version_3 = false;

                let detect_versions = detection_capabilities
                    .get("DetectVersion")
                    .ok_or_else(|| anyhow!("missing DetectVersion"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("expected array"))?;
                for detect_version in detect_versions {
                    let detect_version = detect_version
                        .as_str()
                        .ok_or_else(|| anyhow!("expected string"))?;
                    match detect_version {
                        "V1.0" => {
                            detect_version_1 = true;
                        }
                        "V3.0" => {
                            detect_version_3 = true;
                        }
                        _ => log::warn!("unknown detection version: {detect_version}"),
                    }
                }

                // validate grid size
                let columns = detection_capabilities
                    .get("MotionColumns")
                    .ok_or_else(|| anyhow!("missing MotionColumns"))?
                    .as_u64()
                    .ok_or_else(|| anyhow!("expected number"))? as usize;
                let rows = detection_capabilities
                    .get("MotionRows")
                    .ok_or_else(|| anyhow!("missing MotionRows"))?
                    .as_u64()
                    .ok_or_else(|| anyhow!("expected number"))? as usize;

                ensure!(columns == Grid22x18::COLUMNS && rows == Grid22x18::ROWS);

                // set v1
                if detect_version_1 {
                    patch_object(
                        config,
                        hashmap! {
                            "Level" => json!(3),
                            "Region" => json!(
                                motion_detection
                                    .regions
                                    .get(0)
                                    .map(|region| region.grid)
                                    .unwrap_or_else(Grid22x18::empty)
                                    .as_rows_ltr()
                            )
                        },
                    )
                    .context("patch_object")?;
                }

                // set v3
                if detect_version_3 {
                    let motion_detection_windows = config
                        .get_mut("MotionDetectWindow")
                        .ok_or_else(|| anyhow!("missing MotionDetectWindow"))?
                        .as_array_mut()
                        .ok_or_else(|| anyhow!("expected array"))?;

                    ensure!(motion_detection_windows.len() >= motion_detection.regions.len());

                    motion_detection_windows
                        .iter_mut()
                        .zip(
                            motion_detection
                                .regions
                                .iter()
                                .map(Some)
                                .chain(iter::repeat(None)),
                        )
                        .try_for_each(|(config, region)| -> Result<(), Error> {
                            let config = config
                                .as_object_mut()
                                .ok_or_else(|| anyhow!("expected object"))?;

                            if let Some(region) = region {
                                patch_object(
                                    config,
                                    hashmap! {
                                        "Name" => json!(region.name),
                                        "Region" => json!(region.grid.as_rows_rtl()),
                                        "Window" => json!(region.grid.as_region().as_coords()),
                                        "Sensitive" => json!(region.sensitivity.value),
                                        "Threshold" => json!(region.threshold.value),
                                    },
                                )
                                .context("patch_object")?;
                            } else {
                                patch_object(
                                    config,
                                    hashmap! {
                                        "Region" => json!(Grid22x18::empty().as_rows_rtl()),
                                        "Window" => json!(Grid22x18::empty().as_region().as_coords()),
                                    },
                                )
                                .context("patch_object")?;
                            }

                            Ok(())
                        })?;
                }
            }

            Ok(())
        })
        .await
        .context("config_patch_array_object_with")?;

        Ok(())
    }
    pub async fn detection_smart_motion_configure(
        &mut self,
        detection_capabilities: &serde_json::Map<String, serde_json::Value>,
        smart_motion_detection: Option<SmartMotionDetection>,
    ) -> Result<(), Error> {
        let smart_motion_detection_object = match detection_capabilities.get("SmartMotion") {
            Some(smart_motion_detection_object) => smart_motion_detection_object,
            None => {
                if smart_motion_detection.is_some() {
                    log::warn!("smart motion is not supported, skipping");
                }
                return Ok(());
            }
        };

        let smart_motion_detection_support = smart_motion_detection_object
            .as_object()
            .ok_or_else(|| anyhow!("expected object"))?
            .get("Support")
            .ok_or_else(|| anyhow!("missing Support"))?
            .as_bool()
            .ok_or_else(|| anyhow!("expected bool"))?;

        if !smart_motion_detection_support {
            if smart_motion_detection.is_some() {
                log::warn!("smart motion is not supported, skipping");
            }
            return Ok(());
        }

        self.config_patch_array_object_with("SmartMotionDetect", |config| -> Result<(), Error> {
            if let Some(smart_motion_detection) = smart_motion_detection {
                let value = json!({
                    "Enable": true,
                    "ObjectTypes": {
                        "Human": smart_motion_detection.human,
                        "Vehicle": smart_motion_detection.vehicle,
                    },
                    "Sensitivity": match smart_motion_detection.sensitivity {
                        SmartMotionDetectionSensitivity::Low => "Low",
                        SmartMotionDetectionSensitivity::Medium => "Middle",
                        SmartMotionDetectionSensitivity::High => "High",
                    },
                });
                let value = match value {
                    serde_json::Value::Object(value) => value,
                    _ => panic!(),
                };
                *config = value;
            } else {
                patch_object(
                    config,
                    hashmap! {
                        "Enable" => json!(false),
                    },
                )
                .context("patch_object")?;
            }

            Ok(())
        })
        .await
        .context("config_patch_array_object_with")?;

        Ok(())
    }
    pub async fn detection_video_blind_enable(
        &mut self,
        detection_capabilities: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), Error> {
        let supported = detection_capabilities
            .get("SupportBlind")
            .ok_or_else(|| anyhow!("missing SupportBlind"))?
            .as_bool()
            .ok_or_else(|| anyhow!("expected bool"))?;
        if !supported {
            return Ok(());
        }

        self.config_patch_array_object_with("BlindDetect", |config| -> Result<(), Error> {
            patch_nested_event_handler(config).context("patch_nested_event_handler")?;

            patch_object(
                config,
                hashmap! {
                    "Enable" => json!(true),
                    "Duration" => json!(0),
                },
            )
            .context("patch_object")?;

            Ok(())
        })
        .await
        .context("config_patch_array_object_with")?;

        Ok(())
    }
    pub async fn detection_scene_moved_configure(
        &mut self,
        detection_capabilities: &serde_json::Map<String, serde_json::Value>,
        scene_moved_detection: Option<SceneMovedDetection>,
    ) -> Result<(), Error> {
        let supported = detection_capabilities
            .get("SupportMovedDetect")
            .ok_or_else(|| anyhow!("missing SupportMovedDetect"))?
            .as_bool()
            .ok_or_else(|| anyhow!("expected bool"))?;
        if !supported {
            if scene_moved_detection.is_some() {
                log::warn!("scene moved is not supported, skipping");
            }
            return Ok(());
        }

        self.config_patch_array_object_with("MovedDetect", |config| -> Result<(), Error> {
            patch_nested_event_handler(config).context("patch_nested_event_handler")?;

            patch_object(
                config,
                if let Some(scene_moved_detection) = scene_moved_detection {
                    hashmap! {
                        "Enable" => json!(true),
                        "Sensitivity" => json!(scene_moved_detection.sensitivity.value),
                    }
                } else {
                    hashmap! {
                        "Enable" => json!(false),
                    }
                },
            )
            .context("patch_object")?;

            Ok(())
        })
        .await
        .context("config_patch_array_object_with")?;

        Ok(())
    }
    pub async fn detection_audio_configure(
        &mut self,
        audio_mutation_detection: Option<AudioMutationDetection>,
    ) -> Result<(), Error> {
        // TODO: check if this is covered by something like "get caps"
        let legacy_config_object = self.basic_device_info.web_version
            < (WebVersion {
                major: 3,
                minor: 2,
                revision: 1,
                build: 1053483,
            });

        self.config_patch_with("AudioDetect", |config| -> Result<(), Error> {
            let config = config
                .as_array_mut()
                .ok_or_else(|| anyhow!("expected array"))?;

            if legacy_config_object {
                // anomaly_sensitivity
                // not needed, mutation_sensitivity is the current approach
                ensure!(config.len() == 2);
            } else {
                ensure!(config.len() == 1);
            }

            let config = config.get_mut(0).unwrap();

            let config = config
                .as_object_mut()
                .ok_or_else(|| anyhow!("expected object"))?;

            patch_nested_event_handler(config).context("patch_nested_event_handler")?;

            // mutation_sensitivity
            patch_object(
                config,
                if let Some(audio_mutation_detection) = audio_mutation_detection {
                    hashmap! {
                        "MutationDetect" => json!(true),
                        "MutationThreold" => json!(audio_mutation_detection.sensitivity.value),
                    }
                } else {
                    hashmap! {
                        "MutationDetect" => json!(false),
                    }
                },
            )
            .context("patch_object")?;

            // anomaly_sensitivity
            // maybe in the future

            Ok(())
        })
        .await
        .context("config_patch_with")?;

        Ok(())
    }

    pub async fn configure(
        &mut self,
        factory_reset: bool,
        configuration: Configuration,
    ) -> Result<(), Error> {
        if factory_reset {
            log::trace!("system_factory_reset");
            self.system_factory_reset()
                .await
                .context("system_factory_reset")?;
        } else {
            log::trace!("skipping factory reset");
        }

        // TODO: maybe allow upgrading with incompatible web version?
        // TODO: check web and firmware version after upgrade
        // log::trace!("system_firmware_upgrade");
        // self.system_firmware_upgrade()
        //     .await
        //     .context("system_firmware_upgrade")?;

        log::trace!("system_shared_user");
        self.system_shared_user(configuration.shared_user_password)
            .await
            .context("system_shared_user")?;

        log::trace!("system_arp_ip_setting_disable");
        self.system_arp_ip_setting_disable()
            .await
            .context("system_arp_ip_setting_disable")?;

        log::trace!("system_device_discovery_disable");
        self.system_device_discovery_disable()
            .await
            .context("system_device_discovery_disable")?;

        log::trace!("system_ipv6_enable");
        self.system_ipv6_enable()
            .await
            .context("system_ipv6_enable")?;

        log::trace!("system_multicast_disable");
        self.system_multicast_disable()
            .await
            .context("system_multicast_disable")?;

        log::trace!("system_time_ntp");
        self.system_time_ntp() // break
            .await
            .context("system_time_ntp")?;

        log::trace!("system_snmp_disable");
        self.system_snmp_disable()
            .await
            .context("system_snmp_disable")?;

        log::trace!("system_upnp_disable");
        self.system_upnp_disable()
            .await
            .context("system_upnp_disable")?;

        log::trace!("system_easy4ip_disable");
        self.system_easy4ip_disable()
            .await
            .context("system_easy4ip_disable")?;

        log::trace!("system_bonjour_disable");
        self.system_bonjour_disable()
            .await
            .context("system_bonjour_disable")?;

        log::trace!("system_onvif_disable");
        self.system_onvif_disable()
            .await
            .context("system_onvif_disable")?;

        log::trace!("system_genetec_disable");
        self.system_genetec_disable()
            .await
            .context("system_genetec_disable")?;

        log::trace!("system_lechange_pro_disable");
        self.system_lechange_pro_disable()
            .await
            .context("system_lechange_pro_disable")?;

        log::trace!("system_mobile_phone_platform_disable");
        self.system_mobile_phone_platform_disable()
            .await
            .context("system_mobile_phone_platform_disable")?;

        log::trace!("system_email_disable");
        self.system_email_disable()
            .await
            .context("system_email_disable")?;

        log::trace!("system_hostname_set");
        self.system_hostname_set(&configuration.device_name)
            .await
            .context("system_hostname_set")?;

        log::trace!("system_device_id_name_set");
        self.system_device_id_name_set(configuration.device_id, &configuration.device_name)
            .await
            .context("system_device_id_name_set")?;

        log::trace!("system_old_files_delete_enable");
        self.system_old_files_delete_enable()
            .await
            .context("system_old_files_delete_enable")?;

        log::trace!("system_storage_disable");
        self.system_storage_disable()
            .await
            .context("system_storage_disable")?;

        log::trace!("system_record_disable");
        self.system_record_disable()
            .await
            .context("system_record_disable")?;

        log::trace!("system_ntsc_set");
        self.system_ntsc_set() // break
            .await
            .context("system_ntsc_set")?;

        log::trace!("video_ai_codec_disable");
        self.video_ai_codec_disable()
            .await
            .context("video_ai_codec_disable")?;

        log::trace!("video_quality_configure");
        self.video_quality_configure()
            .await
            .context("video_quality_configure")?;

        log::trace!("video_watermark_disable");
        self.video_watermark_disable()
            .await
            .context("video_watermark_disable")?;

        log::trace!("video_profile_normal_only");
        self.video_profile_normal_only()
            .await
            .context("video_profile_normal_only")?;

        log::trace!("video_orientation_configure");
        self.video_orientation_configure(configuration.video_upside_down)
            .await
            .context("video_orientation_configure")?;

        log::trace!("video_channel_title_configure");
        self.video_channel_title_configure(configuration.channel_title)
            .await
            .context("video_channel_title_configure")?;

        log::trace!("video_privacy_mask_configure");
        self.video_privacy_mask_configure(configuration.privacy_mask)
            .await
            .context("video_privacy_mask_configure")?;

        log::trace!("detection_capabilities_get");
        let detection_capabilities = self
            .detection_capabilities_get()
            .await
            .context("detection_capabilities_get")?;

        log::trace!("detection_external_alarm_disable");
        self.detection_external_alarm_disable()
            .await
            .context("detection_external_alarm_disable")?;

        log::trace!("detection_login_failure_disable");
        self.detection_login_failure_disable()
            .await
            .context("detection_login_failure_disable")?;

        log::trace!("detection_network_conflict_disable");
        self.detection_network_conflict_disable()
            .await
            .context("detection_network_conflict_disable")?;

        log::trace!("detection_network_disconnected_disable");
        self.detection_network_disconnected_disable()
            .await
            .context("detection_network_disconnected_disable")?;

        log::trace!("detection_power_fault_disable");
        self.detection_power_fault_disable()
            .await
            .context("detection_power_fault_disable")?;

        log::trace!("detection_storage_health_alarm_disable");
        self.detection_storage_health_alarm_disable()
            .await
            .context("detection_storage_health_alarm_disable")?;

        log::trace!("detection_motion_configure");
        self.detection_motion_configure(&detection_capabilities, configuration.motion_detection)
            .await
            .context("detection_motion_configure")?;

        log::trace!("detection_smart_motion_configure");
        self.detection_smart_motion_configure(
            &detection_capabilities,
            configuration.smart_motion_detection,
        )
        .await
        .context("detection_smart_motion_configure")?;

        log::trace!("detection_video_blind_enable");
        self.detection_video_blind_enable(&detection_capabilities)
            .await
            .context("detection_video_blind_enable")?;

        log::trace!("detection_scene_moved_configure");
        self.detection_scene_moved_configure(
            &detection_capabilities,
            configuration.scene_moved_detection,
        )
        .await
        .context("detection_scene_moved_configure")?;

        log::trace!("detection_audio_configure");
        self.detection_audio_configure(configuration.audio_mutation_detection)
            .await
            .context("detection_audio_configure")?;

        Ok(())
    }
}

fn patch_object(
    object: &mut serde_json::Map<String, serde_json::Value>,
    patch: HashMap<&str, serde_json::Value>,
) -> Result<(), Error> {
    for (key, value_new) in patch {
        *object
            .get_mut(key)
            .ok_or_else(|| anyhow!("value {} is missing in object", key))? = value_new;
    }

    Ok(())
}
fn patch_nested_event_handler(
    object: &mut serde_json::Map<String, serde_json::Value>
) -> Result<(), Error> {
    let event_handler = object
        .get_mut("EventHandler")
        .ok_or_else(|| anyhow!("EventHandler missing"))?
        .as_object_mut()
        .ok_or_else(|| anyhow!("expected object"))?;

    patch_object(
        event_handler,
        hashmap! {
            "AlarmOutEnable" => json!(false),
            "BeepEnable" => json!(false),
            "ExAlarmOutEnable" => json!(false),
            "FlashEnable" => json!(false),
            "LogEnable" => json!(true),
            "MailEnable" => json!(false),
            "MatrixEnable" => json!(false),
            "MessageEnable" => json!(false),
            "PtzLinkEnable" => json!(false),
            "RecordEnable" => json!(false),
            "SnapshotEnable" => json!(false),
            "TipEnable" => json!(false),
            "TourEnable" => json!(false),
            "VoiceEnable" => json!(false),

            "Dejitter" => json!(5),
        },
    )
    .context("patch_object EventHandler")?;

    Ok(())
}
