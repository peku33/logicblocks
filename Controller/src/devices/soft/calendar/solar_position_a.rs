use crate::{
    datatypes::{
        angle::{AngleNormalized, AngleNormalizedHalfZeroCentered},
        geography::Coordinates3d,
        real::Real,
    },
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use chrono::Utc;
use futures::{future::FutureExt, select};
use maplit::hashmap;
use parking_lot::RwLock;
use serde::Serialize;
use std::{borrow::Cow, time::Duration};

#[derive(Debug)]
pub struct Configuration {
    pub coordinates: Coordinates3d,
    pub calculate_interval: Duration,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    spa: RwLock<Option<spa::SPA2>>,

    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_elevation: signal::state_source::Signal<AngleNormalizedHalfZeroCentered>,
    signal_asimuth: signal::state_source::Signal<AngleNormalized>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            spa: RwLock::new(None),

            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_elevation: signal::state_source::Signal::<AngleNormalizedHalfZeroCentered>::new(
                None,
            ),
            signal_asimuth: signal::state_source::Signal::<AngleNormalized>::new(None),

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    fn calculate(&self) {
        let datetime = Utc::now();

        let spa0 = spa::SPA0::calculate();
        let spa1 = spa::SPA1::calculate(spa0, datetime, spa::DELTA_T_DEFAULT);
        let spa2 = spa::SPA2::calculate(spa1, self.configuration.coordinates);

        let elevation = spa2.topocentric_elevation_angle_without_refraction();
        let asimuth = spa2.topocentric_asimuth();

        let mut signal_sources_changed = false;
        #[allow(unused_assignments)]
        let mut gui_summary_changed = false;

        if self.signal_elevation.set_one(Some(elevation)) {
            signal_sources_changed = true;
        }
        if self.signal_asimuth.set_one(Some(asimuth)) {
            signal_sources_changed = true;
        }

        self.spa.write().replace(spa2);
        gui_summary_changed = true; // jd will always change for example

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    async fn calculate_interval_run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        loop {
            self.calculate();

            select! {
                () = tokio::time::sleep(self.configuration.calculate_interval).fuse() => {},
                () = exit_flag => break,
            }
        }

        Exited
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.calculate_interval_run(exit_flag).await
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/calendar/solar_position_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
    fn as_gui_summary_device_base(&self) -> Option<&dyn devices::gui_summary::DeviceBase> {
        Some(self)
    }
}

#[async_trait]
impl Runnable for Device {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalIdentifier {
    Elevation,
    Azimuth,
}
impl signals::Identifier for SignalIdentifier {}
impl signals::Device for Device {
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        None
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        Some(&self.signals_sources_changed_waker)
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
        hashmap! {
            SignalIdentifier::Elevation => &self.signal_elevation as &dyn signal::Base,
            SignalIdentifier::Azimuth => &self.signal_asimuth as &dyn signal::Base,
        }
    }
}

#[derive(Debug, Serialize)]
struct GuiSummaryInner {
    julian_day: Real,
    elevation: AngleNormalizedHalfZeroCentered,
    asimuth: AngleNormalized,
}
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct GuiSummary {
    inner: Option<GuiSummaryInner>,
}
impl devices::gui_summary::Device for Device {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary;
    fn value(&self) -> Self::Value {
        let spa = self.spa.read();

        let inner = spa.map(|spa| GuiSummaryInner {
            julian_day: spa.as_ref().julian_day(),
            elevation: spa.topocentric_elevation_angle_without_refraction(),
            asimuth: spa.topocentric_asimuth(),
        });

        Self::Value { inner }
    }
}

#[allow(clippy::approx_constant)]
#[allow(clippy::too_many_arguments)]
pub mod spa {
    // based on https://www.nrel.gov/docs/fy08osti/34302.pdf

    use crate::datatypes::{
        angle::{
            AngleNormalized, AngleNormalizedHalf, AngleNormalizedHalfZeroCentered,
            AngleNormalizedZeroCentered,
        },
        geography::Coordinates3d,
        pressure::Pressure,
        real::Real,
        temperature::{Temperature, Unit},
    };
    use arrayvec::ArrayVec;
    use chrono::{DateTime, Datelike, Timelike, Utc};
    use itertools::{zip_eq, Itertools};
    use std::time::Duration;

    pub const DELTA_T_DEFAULT: Duration = Duration::from_secs_f64(32.184);

    #[derive(Clone, Copy, Debug)]
    pub struct SPA0;
    impl SPA0 {
        pub fn calculate() -> Self {
            Self
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct SPA1 {
        spa0: SPA0,

        jd: f64,
        jde: f64,
        jc: f64,
        jce: f64,
        jme: f64,
        l_cap: f64,
        b_cap: f64,
        r_cap: f64,
        theta_cap: f64,
        beta: f64,
        delta_psi: f64,
        delta_epsilon: f64,
        epsilon: f64,
        delta_tau: f64,
        lambda: f64,
        v: f64,
        alpha: f64,
        delta: f64,
        ksi: f64,
    }
    impl SPA1 {
        pub fn calculate(
            spa0: SPA0,
            datetime: DateTime<Utc>,
            delta_t: Duration,
        ) -> Self {
            let delta_t = delta_t.as_secs_f64();

            let jd = jd(datetime);

            // eq. 5
            let jde = jd + (delta_t / 86400.0);

            // eq. 6
            let jc = (jd - 2451545.0) / 36525.0;

            // eq. 7
            let jce = (jde - 2451545.0) / 36525.0;

            // eq. 8
            let jme = jce / 10.0;

            // 3.2.1 - 3.2.6
            let l_cap = l_b_r(
                &[
                    A42_L0_ENTRIES.as_slice(),
                    A42_L1_ENTRIES.as_slice(),
                    A42_L2_ENTRIES.as_slice(),
                    A42_L3_ENTRIES.as_slice(),
                    A42_L4_ENTRIES.as_slice(),
                    A42_L5_ENTRIES.as_slice(),
                ],
                jme,
            );
            let l_cap = normalize_angle_in_radians(l_cap);

            // 3.2.7
            let b_cap = l_b_r(
                &[
                    A42_B0_ENTRIES.as_slice(), // break
                    A42_B1_ENTRIES.as_slice(),
                ],
                jme,
            );

            // 3.2.8
            let r_cap = l_b_r(
                &[
                    A42_R0_ENTRIES.as_slice(),
                    A42_R1_ENTRIES.as_slice(),
                    A42_R2_ENTRIES.as_slice(),
                    A42_R3_ENTRIES.as_slice(),
                    A42_R4_ENTRIES.as_slice(),
                ],
                jme,
            );

            // eq. 13 - 3.3.1 + 3.3.2
            let theta_cap = l_cap + std::f64::consts::PI;
            let theta_cap = normalize_angle_in_radians(theta_cap);

            // eq. 14
            let beta = -b_cap;

            // eq. 15 - eq. 19
            let x_vec = XI_ENTRIES
                .iter()
                .map(|xi_entry| {
                    xi_entry.constant
                        + xi_entry.mul_pow1 * jce
                        + xi_entry.mul_pow2 * jce.powi(2)
                        + jce.powi(3) / (xi_entry.div_pow3 as f64)
                })
                .collect::<ArrayVec<_, XI_ENTRIES_COUNT>>()
                .into_inner()
                .unwrap();

            // supporting for eq. 20 and eq.21
            let sum_product_x_y_i = A43_ENTRIES
                .iter()
                .map(|a43_entry_i| {
                    zip_eq(x_vec.iter(), a43_entry_i.y_vec.iter())
                        .map(|(x_j, y_i_j)| x_j * (*y_i_j as f64))
                        .sum::<f64>()
                        .to_radians()
                })
                .collect::<ArrayVec<_, A43_ENTRIES_COUNT>>()
                .into_inner()
                .unwrap();

            // eq. 20 + eq. 22
            let delta_psi = A43_ENTRIES
                .iter()
                .zip_eq(sum_product_x_y_i.iter())
                .map(|(a43_entry_i, sum_product_x_j_y_i_j)| {
                    ((a43_entry_i.a as f64) + a43_entry_i.b * jce) * sum_product_x_j_y_i_j.sin()
                })
                .sum::<f64>();
            let delta_psi = delta_psi / 36000000.0;
            let delta_psi = delta_psi.to_radians();

            // eq. 21 + eq. 23
            let delta_epsilon = A43_ENTRIES
                .iter()
                .zip_eq(sum_product_x_y_i.iter())
                .map(|(a43_entry_i, sum_product_x_j_y_i_j)| {
                    ((a43_entry_i.c as f64) + a43_entry_i.d * jce) * sum_product_x_j_y_i_j.cos()
                })
                .sum::<f64>();
            let delta_epsilon = delta_epsilon / 36000000.0;
            let delta_epsilon = delta_epsilon.to_radians();

            // eq. 24
            let u_cap = jme / 10.0;
            let epsilon0 = 84381.448 // break
                - 4680.93 * u_cap
                - 1.55 * u_cap.powi(2)
                + 1999.25 * u_cap.powi(3)
                - 51.38 * u_cap.powi(4)
                - 249.67 * u_cap.powi(5)
                - 39.05 * u_cap.powi(6)
                + 7.12 * u_cap.powi(7)
                + 27.87 * u_cap.powi(8)
                + 5.79 * u_cap.powi(9)
                + 2.45 * u_cap.powi(10);
            let epsilon0 = epsilon0 / 3600.0;
            let epsilon0 = epsilon0.to_radians();

            // eq. 25
            let epsilon = epsilon0 + delta_epsilon;

            // eq. 26
            let delta_tau = -20.4898 / (3600.0 * r_cap);
            let delta_tau = delta_tau.to_radians();

            // eq. 27
            let lambda = theta_cap + delta_psi + delta_tau;

            // eq. 28
            let v0 = 280.46061837 + 360.98564736629 * (jd - 2451545.0) + 0.000387933 * jc.powi(2)
                - jc.powi(3) / 38710000.0;
            let v0 = v0.to_radians();
            let v0 = normalize_angle_in_radians(v0);

            // eq. 29
            let v = v0 + delta_psi * epsilon.cos();

            // eq. 30 + 3.9.2
            let alpha =
                (lambda.sin() * epsilon.cos() - beta.tan() * epsilon.sin()).atan2(lambda.cos());
            let alpha = normalize_angle_in_radians(alpha);

            // eq. 31
            let delta =
                (beta.sin() * epsilon.cos() + beta.cos() * epsilon.sin() * lambda.sin()).asin();

            // eq. 33
            let ksi = 8.794 / (3600.0 * r_cap);
            let ksi = ksi.to_radians();

            Self {
                spa0,
                jd,
                jde,
                jc,
                jce,
                jme,
                l_cap,
                b_cap,
                r_cap,
                theta_cap,
                beta,
                delta_psi,
                delta_epsilon,
                epsilon,
                delta_tau,
                lambda,
                v,
                alpha,
                delta,
                ksi,
            }
        }

        pub fn julian_day(&self) -> Real {
            Real::from_f64(self.jd).unwrap()
        }
        // pub fn julian_ephemeris_day(&self) -> f64 {
        //     self.jde
        // }
        // pub fn julian_century(&self) -> f64 {
        //     self.jc
        // }
        // pub fn julian_ephemeris_century(&self) -> f64 {
        //     self.jce
        // }
        // pub fn julian_ephemeris_millenium(&self) -> f64 {
        //     self.jme
        // }
        pub fn heliocentric_longitude(&self) -> AngleNormalized {
            AngleNormalized::from_radians(self.l_cap).unwrap()
        }
        pub fn heliocentric_latitude(&self) -> AngleNormalizedZeroCentered {
            AngleNormalizedZeroCentered::from_radians(self.b_cap).unwrap()
        }
        // pub fn radius_vector(&self) -> f64 {
        //     self.r_cap // Astronomical Units
        // }
        pub fn geocentric_longitude(&self) -> AngleNormalized {
            AngleNormalized::from_radians(self.theta_cap).unwrap()
        }
        pub fn geocentric_latitude(&self) -> AngleNormalizedZeroCentered {
            AngleNormalizedZeroCentered::from_radians(self.beta).unwrap()
        }
        // pub fn nutation_in_longitude(&self) -> f64 {
        //     self.delta_psi
        // }
        // pub fn nutation_in_obliquity(&self) -> f64 {
        //     self.delta_epsilon
        // }
        // pub fn obliquity_of_ecliptic(&self) -> f64 {
        //     self.epsilon
        // }
        // pub fn abberation_correction(&self) -> f64 {
        //     self.delta_tau
        // }
        pub fn apparent_sun_longitude(&self) -> AngleNormalized {
            AngleNormalized::from_radians(self.lambda).unwrap()
        }
        pub fn apparent_sidereal_time_greenwich(&self) -> AngleNormalized {
            AngleNormalized::from_radians(self.v).unwrap()
        }
        pub fn geocentric_sun_right_ascention(&self) -> AngleNormalized {
            AngleNormalized::from_radians(self.alpha).unwrap()
        }
        pub fn geocentric_sun_declination(&self) -> AngleNormalizedHalfZeroCentered {
            AngleNormalizedHalfZeroCentered::from_radians(self.delta).unwrap()
        }
        // pub fn equatorial_horizontal_parallax(&self) -> f64 {
        //     self.ksi
        // }
    }
    impl AsRef<SPA0> for SPA1 {
        fn as_ref(&self) -> &SPA0 {
            &self.spa0
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct SPA2 {
        spa1: SPA1,

        h_cap: f64,
        delta_alpha: f64,
        alpha_prim: f64,
        delta_prim: f64,
        h_cap_prim: f64,
        e0: f64,
        gamma_cap: f64,
        phi_cap: f64,
    }
    impl SPA2 {
        pub fn calculate(
            spa1: SPA1,
            coordinates: Coordinates3d,
        ) -> Self {
            let SPA1 {
                v,
                alpha,
                delta,
                ksi,
                ..
            } = spa1;

            let sigma = coordinates.coordinates_2d.longitude.to_radians();
            let phi = coordinates.coordinates_2d.latitude.to_radians();
            let e_cap = coordinates.elevation.to_meters();

            // eq. 32
            let h_cap = v + sigma - alpha;
            let h_cap = normalize_angle_in_radians(h_cap);

            // eq. 34
            let u = (0.99664719 * phi.tan()).atan();

            // eq. 35
            let x = u.cos() + (e_cap / 6378140.0) * phi.cos();

            // eq. 36
            let y = 0.99664719 * u.sin() + (e_cap / 6378140.0) * phi.sin();

            // eq. 37
            let delta_alpha =
                (-x * ksi.sin() * h_cap.sin()).atan2(delta.cos() - x * ksi.sin() * h_cap.cos());
            let delta_alpha = delta_alpha.to_radians();

            // eq. 38
            let alpha_prim = alpha + delta_alpha;

            // eq. 39
            let delta_prim = ((delta.sin() - y * ksi.sin()) * delta_alpha.cos())
                .atan2(delta.cos() - x * ksi.sin() * h_cap.cos());

            // eq. 40
            let h_cap_prim = h_cap - delta_alpha;

            // eq. 41
            let e0 = (phi.sin() * delta_prim.sin()
                + phi.cos() * delta_prim.cos() * h_cap_prim.cos())
            .asin();

            // eq. 45
            let gamma_cap = h_cap_prim
                .sin()
                .atan2(h_cap_prim.cos() * phi.sin() - delta_prim.tan() * phi.cos());
            let gamma_cap = normalize_angle_in_radians(gamma_cap);

            // eq. 46
            let phi_cap = gamma_cap + std::f64::consts::PI;
            let phi_cap = normalize_angle_in_radians(phi_cap);

            Self {
                spa1,
                h_cap,
                delta_alpha,
                alpha_prim,
                delta_prim,
                h_cap_prim,
                e0,
                gamma_cap,
                phi_cap,
            }
        }

        pub fn observer_local_hour_angle(&self) -> AngleNormalized {
            AngleNormalized::from_radians(self.h_cap).unwrap()
        }
        pub fn parallax_in_sun_right_ascention(&self) -> AngleNormalizedZeroCentered {
            AngleNormalizedZeroCentered::from_radians(self.delta_alpha).unwrap()
        }
        pub fn topocentric_sun_right_ascention(&self) -> AngleNormalizedZeroCentered {
            AngleNormalizedZeroCentered::from_radians(self.alpha_prim).unwrap()
        }
        pub fn topocentric_sun_declination(&self) -> AngleNormalizedZeroCentered {
            AngleNormalizedZeroCentered::from_radians(self.delta_prim).unwrap()
        }
        pub fn topocentric_local_hour_angle(&self) -> AngleNormalized {
            AngleNormalized::from_radians(self.h_cap_prim).unwrap()
        }
        pub fn topocentric_elevation_angle_without_refraction(
            &self
        ) -> AngleNormalizedHalfZeroCentered {
            AngleNormalizedHalfZeroCentered::from_radians(self.e0).unwrap()
        }
        pub fn topocentric_asimuth_astronomers(&self) -> AngleNormalized {
            AngleNormalized::from_radians(self.gamma_cap).unwrap()
        }
        pub fn topocentric_asimuth(&self) -> AngleNormalized {
            AngleNormalized::from_radians(self.phi_cap).unwrap()
        }
    }
    impl AsRef<SPA1> for SPA2 {
        fn as_ref(&self) -> &SPA1 {
            &self.spa1
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct SPA3 {
        spa2: SPA2,

        delta_e: f64,
        e: f64,
        theta: f64,
    }
    impl SPA3 {
        pub fn calculate(
            spa2: SPA2,
            pressure: Pressure,
            temperature: Temperature,
        ) -> Self {
            let SPA2 { e0, .. } = spa2;

            let p = pressure.to_millibars_hectopascals();
            let t = temperature.to_unit(Unit::Celsius);

            // eq. 42
            let delta_e = 1.0
                * (p / 1010.0)
                * (283.0 / (273.0 + t))
                * (1.02
                    / (60.0
                        * (e0.to_degrees() + (10.3 / (e0.to_degrees() + 5.11)))
                            .to_radians()
                            .tan()));
            let delta_e = delta_e.to_radians();

            // eq. 43
            let e = e0 + delta_e;

            // eq. 44
            let theta = std::f64::consts::PI / 2.0 - e;

            Self {
                spa2,
                delta_e,
                e,
                theta,
            }
        }

        // pub fn topocentric_elevation_angle_refraction_correction(&self) -> f64 {
        //     self.delta_e
        // }
        pub fn topocentric_elevation_angle(&self) -> AngleNormalizedHalfZeroCentered {
            AngleNormalizedHalfZeroCentered::from_radians(self.e).unwrap()
        }
        pub fn topocentric_zenith_angle(&self) -> AngleNormalizedHalfZeroCentered {
            AngleNormalizedHalfZeroCentered::from_radians(self.theta).unwrap()
        }
    }
    impl AsRef<SPA2> for SPA3 {
        fn as_ref(&self) -> &SPA2 {
            &self.spa2
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct SPA4 {
        spa3: SPA3,

        i_cap: f64,
    }
    impl SPA4 {
        pub fn calculate(
            spa3: SPA3,
            slope: f64, // slope of the surface measured from the horizontal plane in radians
            azimuth: f64, // surface azimuth rotation angle, measured from south to the projection of the surface normal on the horizontal plane, positive or negative if oriented west or east from south, respectively in radians
        ) -> Self {
            let SPA3 { spa2, theta, .. } = spa3;
            let SPA2 { gamma_cap, .. } = spa2;

            let omega = slope;
            let gamma = azimuth;

            // eq. 47
            let i_cap = (theta.cos() * omega.cos()
                + omega.sin() * theta.sin() * (gamma_cap - gamma).cos())
            .acos();

            Self { spa3, i_cap }
        }

        pub fn incidence_angle(&self) -> AngleNormalizedHalf {
            AngleNormalizedHalf::from_radians(self.i_cap).unwrap()
        }
    }
    impl AsRef<SPA3> for SPA4 {
        fn as_ref(&self) -> &SPA3 {
            &self.spa3
        }
    }

    #[cfg(test)]
    mod tests_spa {
        use super::{SPA0, SPA1, SPA2, SPA3, SPA4};
        use crate::datatypes::{
            geography::{Coordinates2d, Coordinates3d, Elevation, Latitude, Longitude},
            pressure::Pressure,
            temperature::{Temperature, Unit},
        };
        use approx::assert_relative_eq;
        use chrono::{FixedOffset, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
        use std::time::Duration;

        #[test]
        fn test_table_1() {
            let spa0 = SPA0::calculate();

            let date = NaiveDate::from_ymd_opt(2003, 10, 17).unwrap();
            let time = NaiveTime::from_hms_opt(12, 30, 30).unwrap();
            let timezone = -7;
            let delta_t = Duration::from_secs_f64(67.0);
            let datetime = FixedOffset::east_opt(3600 * timezone)
                .unwrap()
                .from_local_datetime(&date.and_time(time))
                .unwrap()
                .with_timezone(&Utc);
            assert_eq!(datetime.hour(), 19);

            let spa1 = SPA1::calculate(spa0, datetime, delta_t);
            assert_relative_eq!(spa1.jd, 2452930.312847, epsilon = 1e-5);
            assert_relative_eq!(spa1.l_cap, 24.0182616917_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa1.b_cap, -0.0001011219_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa1.r_cap, 0.9965422974, epsilon = 1e-5);
            assert_relative_eq!(
                spa1.theta_cap,
                204.0182616917_f64.to_radians(),
                epsilon = 1e-5
            );
            assert_relative_eq!(spa1.beta, 0.0001011219_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa1.delta_psi, -0.00399840_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(
                spa1.delta_epsilon,
                0.00166657_f64.to_radians(),
                epsilon = 1e-5
            );
            assert_relative_eq!(spa1.epsilon, 23.440465_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa1.lambda, 204.0085519281_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa1.alpha, 202.22741_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa1.delta, -9.31434_f64.to_radians(), epsilon = 1e-5);

            let longitude = Longitude::from_degrees(-105.1786).unwrap();
            let latitude = Latitude::from_degrees(39.742476).unwrap();
            let elevation = Elevation::from_meters(1830.14).unwrap();
            let coordinates_2d = Coordinates2d {
                latitude,
                longitude,
            };
            let coordinates_3d = Coordinates3d {
                coordinates_2d,
                elevation,
            };

            let spa2 = SPA2::calculate(spa1, coordinates_3d);

            assert_relative_eq!(spa2.h_cap, 11.105900_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa2.h_cap_prim, 11.10629_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa2.alpha_prim, 202.22704_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa2.delta_prim, -9.316179_f64.to_radians(), epsilon = 1e-5);
            assert_relative_eq!(spa2.phi_cap, 194.34024_f64.to_radians(), epsilon = 1e-5);

            let pressure = Pressure::from_millibars_hectopascals(820.0).unwrap();
            let temperature = Temperature::new(Unit::Celsius, 11.0).unwrap();

            let spa3 = SPA3::calculate(spa2, pressure, temperature);
            assert_relative_eq!(spa3.theta, 50.11162_f64.to_radians(), epsilon = 1e-5);

            let slope = 30.0_f64.to_radians();
            let azimuth = -10.0_f64.to_radians();

            let spa4 = SPA4::calculate(spa3, slope, azimuth);
            assert_relative_eq!(spa4.i_cap, 25.18700_f64.to_radians(), epsilon = 1e-5);
        }
    }

    fn jd(datetime: DateTime<Utc>) -> f64 {
        let mut y = datetime.year();
        let mut m = datetime.month();
        if m <= 2 {
            y -= 1;
            m += 12;
        }
        let y = y;
        let m = m;

        let d = datetime.day() as f64
            + datetime.num_seconds_from_midnight() as f64 / (60.0 * 60.0 * 24.0);

        // eq. 4
        let mut jd =
            (365.25 * (y + 4716) as f64).floor() + (30.6001 * (m + 1) as f64).floor() + d - 1524.5;

        if jd >= 2299160.0 {
            let a = (y as f64 / 100.0).floor();
            let b_cap = 2.0 - a + (a / 4.0).floor();
            jd += b_cap
        }
        let jd = jd;

        jd
    }
    #[cfg(test)]
    mod tests_jd {
        use super::jd;
        use chrono::{TimeZone, Utc};

        #[test]
        fn test_jd_table_1() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap()),
                2451545.0
            );
        }
        #[test]
        fn test_jd_table_2() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(1999, 1, 1, 0, 0, 0).unwrap()),
                2451179.5
            );
        }
        #[test]
        fn test_jd_table_3() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(1987, 1, 27, 0, 0, 0).unwrap()),
                2446822.5
            );
        }
        #[test]
        fn test_jd_table_4() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(1987, 6, 19, 12, 0, 0).unwrap()),
                2446966.0
            );
        }
        #[test]
        fn test_jd_table_5() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(1988, 1, 27, 0, 0, 0).unwrap()),
                2447187.5
            );
        }
        #[test]
        fn test_jd_table_6() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(1988, 6, 19, 12, 0, 0).unwrap()),
                2447332.0
            );
        }
        #[test]
        fn test_jd_table_7() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(1900, 1, 1, 0, 0, 0).unwrap()),
                2415020.5
            );
        }
        #[test]
        fn test_jd_table_8() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(1600, 1, 1, 0, 0, 0).unwrap()),
                2305447.5
            );
        }
        #[test]
        fn test_jd_table_9() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(1600, 12, 31, 0, 0, 0).unwrap()),
                2305812.5
            );
        }
        #[test]
        fn test_jd_table_10() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(837, 4, 10, 7, 12, 0).unwrap()),
                2026871.8
            );
        }
        #[test]
        fn test_jd_table_11() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(-123, 12, 31, 0, 0, 0).unwrap()),
                1676496.5
            );
        }
        #[test]
        fn test_jd_table_12() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(-122, 1, 1, 0, 0, 0).unwrap()),
                1676497.5
            );
        }
        #[test]
        fn test_jd_table_13() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(-1000, 7, 12, 12, 0, 0).unwrap()),
                1356001.0
            );
        }
        // #[test]
        // fn test_jd_table_14() {
        //     assert_eq!(jd(Utc.with_ymd_and_hms(-1000, 2, 29, 0, 0, 0)), 1355866.5);
        // }
        #[test]
        fn test_jd_table_15() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(-1001, 8, 17, 21, 36, 0).unwrap()),
                1355671.4
            );
        }
        #[test]
        fn test_jd_table_16() {
            assert_eq!(
                jd(Utc.with_ymd_and_hms(-4712, 1, 1, 12, 0, 0).unwrap()),
                0.0
            );
        }
    }

    // eq. 9 - eq. 12
    // 3.2.1 - 3.2.8
    fn l_b_r(
        inputs: &[&[A42Entry]],
        jme: f64,
    ) -> f64 {
        let l_b_r = inputs.iter().map(|a42_entries_l_i| {
            a42_entries_l_i
                .iter()
                .map(|a42_entry_i| {
                    (a42_entry_i.a_cap as f64) * (a42_entry_i.b_cap + a42_entry_i.c_cap * jme).cos()
                })
                .sum::<f64>()
        });

        let l_b_r = l_b_r
            .enumerate()
            .map(|(i, l_i)| l_i * jme.powi(i as i32))
            .sum::<f64>()
            / 1e8;

        l_b_r
    }

    fn mod_360(input: f64) -> f64 {
        const DIVISOR: f64 = 360.0;
        (input % DIVISOR + DIVISOR) % DIVISOR
    }
    fn normalize_angle_in_radians(input: f64) -> f64 {
        const DIVISOR: f64 = 2.0 * std::f64::consts::PI;
        (input % DIVISOR + DIVISOR) % DIVISOR
    }

    // table A4.2
    #[derive(Debug)]
    struct A42Entry {
        a_cap: u64,
        b_cap: f64,
        c_cap: f64,
    }

    const A42_L0_ENTRIES_COUNT: usize = 64;
    static A42_L0_ENTRIES: [A42Entry; A42_L0_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 175347046,
            b_cap: 0.0,
            c_cap: 0.0,
        },
        A42Entry {
            a_cap: 3341656,
            b_cap: 4.6692568,
            c_cap: 6283.07585,
        },
        A42Entry {
            a_cap: 34894,
            b_cap: 4.6261,
            c_cap: 12566.1517,
        },
        A42Entry {
            a_cap: 3497,
            b_cap: 2.7441,
            c_cap: 5753.3849,
        },
        A42Entry {
            a_cap: 3418,
            b_cap: 2.8289,
            c_cap: 3.5231,
        },
        A42Entry {
            a_cap: 3136,
            b_cap: 3.6277,
            c_cap: 77713.7715,
        },
        A42Entry {
            a_cap: 2676,
            b_cap: 4.4181,
            c_cap: 7860.4194,
        },
        A42Entry {
            a_cap: 2343,
            b_cap: 6.1352,
            c_cap: 3930.2097,
        },
        A42Entry {
            a_cap: 1324,
            b_cap: 0.7425,
            c_cap: 11506.7698,
        },
        A42Entry {
            a_cap: 1273,
            b_cap: 2.0371,
            c_cap: 529.691,
        },
        A42Entry {
            a_cap: 1199,
            b_cap: 1.1096,
            c_cap: 1577.3435,
        },
        A42Entry {
            a_cap: 990,
            b_cap: 5.233,
            c_cap: 5884.927,
        },
        A42Entry {
            a_cap: 902,
            b_cap: 2.045,
            c_cap: 26.298,
        },
        A42Entry {
            a_cap: 857,
            b_cap: 3.508,
            c_cap: 398.149,
        },
        A42Entry {
            a_cap: 780,
            b_cap: 1.179,
            c_cap: 5223.694,
        },
        A42Entry {
            a_cap: 753,
            b_cap: 2.533,
            c_cap: 5507.553,
        },
        A42Entry {
            a_cap: 505,
            b_cap: 4.583,
            c_cap: 18849.228,
        },
        A42Entry {
            a_cap: 492,
            b_cap: 4.205,
            c_cap: 775.523,
        },
        A42Entry {
            a_cap: 357,
            b_cap: 2.92,
            c_cap: 0.067,
        },
        A42Entry {
            a_cap: 317,
            b_cap: 5.849,
            c_cap: 11790.629,
        },
        A42Entry {
            a_cap: 284,
            b_cap: 1.899,
            c_cap: 796.298,
        },
        A42Entry {
            a_cap: 271,
            b_cap: 0.315,
            c_cap: 10977.079,
        },
        A42Entry {
            a_cap: 243,
            b_cap: 0.345,
            c_cap: 5486.778,
        },
        A42Entry {
            a_cap: 206,
            b_cap: 4.806,
            c_cap: 2544.314,
        },
        A42Entry {
            a_cap: 205,
            b_cap: 1.869,
            c_cap: 5573.143,
        },
        A42Entry {
            a_cap: 202,
            b_cap: 2.458,
            c_cap: 6069.777,
        },
        A42Entry {
            a_cap: 156,
            b_cap: 0.833,
            c_cap: 213.299,
        },
        A42Entry {
            a_cap: 132,
            b_cap: 3.411,
            c_cap: 2942.463,
        },
        A42Entry {
            a_cap: 126,
            b_cap: 1.083,
            c_cap: 20.775,
        },
        A42Entry {
            a_cap: 115,
            b_cap: 0.645,
            c_cap: 0.98,
        },
        A42Entry {
            a_cap: 103,
            b_cap: 0.636,
            c_cap: 4694.003,
        },
        A42Entry {
            a_cap: 102,
            b_cap: 0.976,
            c_cap: 15720.839,
        },
        A42Entry {
            a_cap: 102,
            b_cap: 4.267,
            c_cap: 7.114,
        },
        A42Entry {
            a_cap: 99,
            b_cap: 6.21,
            c_cap: 2146.17,
        },
        A42Entry {
            a_cap: 98,
            b_cap: 0.68,
            c_cap: 155.42,
        },
        A42Entry {
            a_cap: 86,
            b_cap: 5.98,
            c_cap: 161000.69,
        },
        A42Entry {
            a_cap: 85,
            b_cap: 1.3,
            c_cap: 6275.96,
        },
        A42Entry {
            a_cap: 85,
            b_cap: 3.67,
            c_cap: 71430.7,
        },
        A42Entry {
            a_cap: 80,
            b_cap: 1.81,
            c_cap: 17260.15,
        },
        A42Entry {
            a_cap: 79,
            b_cap: 3.04,
            c_cap: 12036.46,
        },
        A42Entry {
            a_cap: 75,
            b_cap: 1.76,
            c_cap: 5088.63,
        },
        A42Entry {
            a_cap: 74,
            b_cap: 3.5,
            c_cap: 3154.69,
        },
        A42Entry {
            a_cap: 74,
            b_cap: 4.68,
            c_cap: 801.82,
        },
        A42Entry {
            a_cap: 70,
            b_cap: 0.83,
            c_cap: 9437.76,
        },
        A42Entry {
            a_cap: 62,
            b_cap: 3.98,
            c_cap: 8827.39,
        },
        A42Entry {
            a_cap: 61,
            b_cap: 1.82,
            c_cap: 7084.9,
        },
        A42Entry {
            a_cap: 57,
            b_cap: 2.78,
            c_cap: 6286.6,
        },
        A42Entry {
            a_cap: 56,
            b_cap: 4.39,
            c_cap: 14143.5,
        },
        A42Entry {
            a_cap: 56,
            b_cap: 3.47,
            c_cap: 6279.55,
        },
        A42Entry {
            a_cap: 52,
            b_cap: 0.19,
            c_cap: 12139.55,
        },
        A42Entry {
            a_cap: 52,
            b_cap: 1.33,
            c_cap: 1748.02,
        },
        A42Entry {
            a_cap: 51,
            b_cap: 0.28,
            c_cap: 5856.48,
        },
        A42Entry {
            a_cap: 49,
            b_cap: 0.49,
            c_cap: 1194.45,
        },
        A42Entry {
            a_cap: 41,
            b_cap: 5.37,
            c_cap: 8429.24,
        },
        A42Entry {
            a_cap: 41,
            b_cap: 2.4,
            c_cap: 19651.05,
        },
        A42Entry {
            a_cap: 39,
            b_cap: 6.17,
            c_cap: 10447.39,
        },
        A42Entry {
            a_cap: 37,
            b_cap: 6.04,
            c_cap: 10213.29,
        },
        A42Entry {
            a_cap: 37,
            b_cap: 2.57,
            c_cap: 1059.38,
        },
        A42Entry {
            a_cap: 36,
            b_cap: 1.71,
            c_cap: 2352.87,
        },
        A42Entry {
            a_cap: 36,
            b_cap: 1.78,
            c_cap: 6812.77,
        },
        A42Entry {
            a_cap: 33,
            b_cap: 0.59,
            c_cap: 17789.85,
        },
        A42Entry {
            a_cap: 30,
            b_cap: 0.44,
            c_cap: 83996.85,
        },
        A42Entry {
            a_cap: 30,
            b_cap: 2.74,
            c_cap: 1349.87,
        },
        A42Entry {
            a_cap: 25,
            b_cap: 3.16,
            c_cap: 4690.48,
        },
    ];

    const A42_L1_ENTRIES_COUNT: usize = 34;
    static A42_L1_ENTRIES: [A42Entry; A42_L1_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 628331966747,
            b_cap: 0.0,
            c_cap: 0.0,
        },
        A42Entry {
            a_cap: 206059,
            b_cap: 2.678235,
            c_cap: 6283.07585,
        },
        A42Entry {
            a_cap: 4303,
            b_cap: 2.6351,
            c_cap: 12566.1517,
        },
        A42Entry {
            a_cap: 425,
            b_cap: 1.59,
            c_cap: 3.523,
        },
        A42Entry {
            a_cap: 119,
            b_cap: 5.796,
            c_cap: 26.298,
        },
        A42Entry {
            a_cap: 109,
            b_cap: 2.966,
            c_cap: 1577.344,
        },
        A42Entry {
            a_cap: 93,
            b_cap: 2.59,
            c_cap: 18849.23,
        },
        A42Entry {
            a_cap: 72,
            b_cap: 1.14,
            c_cap: 529.69,
        },
        A42Entry {
            a_cap: 68,
            b_cap: 1.87,
            c_cap: 398.15,
        },
        A42Entry {
            a_cap: 67,
            b_cap: 4.41,
            c_cap: 5507.55,
        },
        A42Entry {
            a_cap: 59,
            b_cap: 2.89,
            c_cap: 5223.69,
        },
        A42Entry {
            a_cap: 56,
            b_cap: 2.17,
            c_cap: 155.42,
        },
        A42Entry {
            a_cap: 45,
            b_cap: 0.4,
            c_cap: 796.3,
        },
        A42Entry {
            a_cap: 36,
            b_cap: 0.47,
            c_cap: 775.52,
        },
        A42Entry {
            a_cap: 29,
            b_cap: 2.65,
            c_cap: 7.11,
        },
        A42Entry {
            a_cap: 21,
            b_cap: 5.34,
            c_cap: 0.98,
        },
        A42Entry {
            a_cap: 19,
            b_cap: 1.85,
            c_cap: 5486.78,
        },
        A42Entry {
            a_cap: 19,
            b_cap: 4.97,
            c_cap: 213.3,
        },
        A42Entry {
            a_cap: 17,
            b_cap: 2.99,
            c_cap: 6275.96,
        },
        A42Entry {
            a_cap: 16,
            b_cap: 0.03,
            c_cap: 2544.31,
        },
        A42Entry {
            a_cap: 16,
            b_cap: 1.43,
            c_cap: 2146.17,
        },
        A42Entry {
            a_cap: 15,
            b_cap: 1.21,
            c_cap: 10977.08,
        },
        A42Entry {
            a_cap: 12,
            b_cap: 2.83,
            c_cap: 1748.02,
        },
        A42Entry {
            a_cap: 12,
            b_cap: 3.26,
            c_cap: 5088.63,
        },
        A42Entry {
            a_cap: 12,
            b_cap: 5.27,
            c_cap: 1194.45,
        },
        A42Entry {
            a_cap: 12,
            b_cap: 2.08,
            c_cap: 4694.0,
        },
        A42Entry {
            a_cap: 11,
            b_cap: 0.77,
            c_cap: 553.57,
        },
        A42Entry {
            a_cap: 10,
            b_cap: 1.3,
            c_cap: 6286.6,
        },
        A42Entry {
            a_cap: 10,
            b_cap: 4.24,
            c_cap: 1349.87,
        },
        A42Entry {
            a_cap: 9,
            b_cap: 2.7,
            c_cap: 242.73,
        },
        A42Entry {
            a_cap: 9,
            b_cap: 5.64,
            c_cap: 951.72,
        },
        A42Entry {
            a_cap: 8,
            b_cap: 5.3,
            c_cap: 2352.87,
        },
        A42Entry {
            a_cap: 6,
            b_cap: 2.65,
            c_cap: 9437.76,
        },
        A42Entry {
            a_cap: 6,
            b_cap: 4.67,
            c_cap: 4690.48,
        },
    ];

    const A42_L2_ENTRIES_COUNT: usize = 20;
    static A42_L2_ENTRIES: [A42Entry; A42_L2_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 52919,
            b_cap: 0.0,
            c_cap: 0.0,
        },
        A42Entry {
            a_cap: 8720,
            b_cap: 1.0721,
            c_cap: 6283.0758,
        },
        A42Entry {
            a_cap: 309,
            b_cap: 0.867,
            c_cap: 12566.152,
        },
        A42Entry {
            a_cap: 27,
            b_cap: 0.05,
            c_cap: 3.52,
        },
        A42Entry {
            a_cap: 16,
            b_cap: 5.19,
            c_cap: 26.3,
        },
        A42Entry {
            a_cap: 16,
            b_cap: 3.68,
            c_cap: 155.42,
        },
        A42Entry {
            a_cap: 10,
            b_cap: 0.76,
            c_cap: 18849.23,
        },
        A42Entry {
            a_cap: 9,
            b_cap: 2.06,
            c_cap: 77713.77,
        },
        A42Entry {
            a_cap: 7,
            b_cap: 0.83,
            c_cap: 775.52,
        },
        A42Entry {
            a_cap: 5,
            b_cap: 4.66,
            c_cap: 1577.34,
        },
        A42Entry {
            a_cap: 4,
            b_cap: 1.03,
            c_cap: 7.11,
        },
        A42Entry {
            a_cap: 4,
            b_cap: 3.44,
            c_cap: 5573.14,
        },
        A42Entry {
            a_cap: 3,
            b_cap: 5.14,
            c_cap: 796.3,
        },
        A42Entry {
            a_cap: 3,
            b_cap: 6.05,
            c_cap: 5507.55,
        },
        A42Entry {
            a_cap: 3,
            b_cap: 1.19,
            c_cap: 242.73,
        },
        A42Entry {
            a_cap: 3,
            b_cap: 6.12,
            c_cap: 529.69,
        },
        A42Entry {
            a_cap: 3,
            b_cap: 0.31,
            c_cap: 398.15,
        },
        A42Entry {
            a_cap: 3,
            b_cap: 2.28,
            c_cap: 553.57,
        },
        A42Entry {
            a_cap: 2,
            b_cap: 4.38,
            c_cap: 5223.69,
        },
        A42Entry {
            a_cap: 2,
            b_cap: 3.75,
            c_cap: 0.98,
        },
    ];

    const A42_L3_ENTRIES_COUNT: usize = 7;
    static A42_L3_ENTRIES: [A42Entry; A42_L3_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 289,
            b_cap: 5.844,
            c_cap: 6283.076,
        },
        A42Entry {
            a_cap: 35,
            b_cap: 0.0,
            c_cap: 0.0,
        },
        A42Entry {
            a_cap: 17,
            b_cap: 5.49,
            c_cap: 12566.15,
        },
        A42Entry {
            a_cap: 3,
            b_cap: 5.2,
            c_cap: 155.42,
        },
        A42Entry {
            a_cap: 1,
            b_cap: 4.72,
            c_cap: 3.52,
        },
        A42Entry {
            a_cap: 1,
            b_cap: 5.3,
            c_cap: 18849.23,
        },
        A42Entry {
            a_cap: 1,
            b_cap: 5.97,
            c_cap: 242.73,
        },
    ];

    const A42_L4_ENTRIES_COUNT: usize = 3;
    static A42_L4_ENTRIES: [A42Entry; A42_L4_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 114,
            b_cap: 3.142,
            c_cap: 0.0,
        },
        A42Entry {
            a_cap: 8,
            b_cap: 4.13,
            c_cap: 6283.08,
        },
        A42Entry {
            a_cap: 1,
            b_cap: 3.84,
            c_cap: 12566.15,
        },
    ];

    const A42_L5_ENTRIES_COUNT: usize = 1;
    static A42_L5_ENTRIES: [A42Entry; A42_L5_ENTRIES_COUNT] = [A42Entry {
        a_cap: 1,
        b_cap: 3.14,
        c_cap: 0.0,
    }];

    const A42_B0_ENTRIES_COUNT: usize = 5;
    static A42_B0_ENTRIES: [A42Entry; A42_B0_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 280,
            b_cap: 3.199,
            c_cap: 84334.662,
        },
        A42Entry {
            a_cap: 102,
            b_cap: 5.422,
            c_cap: 5507.553,
        },
        A42Entry {
            a_cap: 80,
            b_cap: 3.88,
            c_cap: 5223.69,
        },
        A42Entry {
            a_cap: 44,
            b_cap: 3.7,
            c_cap: 2352.87,
        },
        A42Entry {
            a_cap: 32,
            b_cap: 4.0,
            c_cap: 1577.34,
        },
    ];

    const A42_B1_ENTRIES_COUNT: usize = 2;
    static A42_B1_ENTRIES: [A42Entry; A42_B1_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 9,
            b_cap: 3.9,
            c_cap: 5507.55,
        },
        A42Entry {
            a_cap: 6,
            b_cap: 1.73,
            c_cap: 5223.69,
        },
    ];

    const A42_R0_ENTRIES_COUNT: usize = 40;
    static A42_R0_ENTRIES: [A42Entry; A42_R0_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 100013989,
            b_cap: 0.0,
            c_cap: 0.0,
        },
        A42Entry {
            a_cap: 1670700,
            b_cap: 3.0984635,
            c_cap: 6283.07585,
        },
        A42Entry {
            a_cap: 13956,
            b_cap: 3.05525,
            c_cap: 12566.1517,
        },
        A42Entry {
            a_cap: 3084,
            b_cap: 5.1985,
            c_cap: 77713.7715,
        },
        A42Entry {
            a_cap: 1628,
            b_cap: 1.1739,
            c_cap: 5753.3849,
        },
        A42Entry {
            a_cap: 1576,
            b_cap: 2.8469,
            c_cap: 7860.4194,
        },
        A42Entry {
            a_cap: 925,
            b_cap: 5.453,
            c_cap: 11506.77,
        },
        A42Entry {
            a_cap: 542,
            b_cap: 4.564,
            c_cap: 3930.21,
        },
        A42Entry {
            a_cap: 472,
            b_cap: 3.661,
            c_cap: 5884.927,
        },
        A42Entry {
            a_cap: 346,
            b_cap: 0.964,
            c_cap: 5507.553,
        },
        A42Entry {
            a_cap: 329,
            b_cap: 5.9,
            c_cap: 5223.694,
        },
        A42Entry {
            a_cap: 307,
            b_cap: 0.299,
            c_cap: 5573.143,
        },
        A42Entry {
            a_cap: 243,
            b_cap: 4.273,
            c_cap: 11790.629,
        },
        A42Entry {
            a_cap: 212,
            b_cap: 5.847,
            c_cap: 1577.344,
        },
        A42Entry {
            a_cap: 186,
            b_cap: 5.022,
            c_cap: 10977.079,
        },
        A42Entry {
            a_cap: 175,
            b_cap: 3.012,
            c_cap: 18849.228,
        },
        A42Entry {
            a_cap: 110,
            b_cap: 5.055,
            c_cap: 5486.778,
        },
        A42Entry {
            a_cap: 98,
            b_cap: 0.89,
            c_cap: 6069.78,
        },
        A42Entry {
            a_cap: 86,
            b_cap: 5.69,
            c_cap: 15720.84,
        },
        A42Entry {
            a_cap: 86,
            b_cap: 1.27,
            c_cap: 161000.69,
        },
        A42Entry {
            a_cap: 65,
            b_cap: 0.27,
            c_cap: 17260.15,
        },
        A42Entry {
            a_cap: 63,
            b_cap: 0.92,
            c_cap: 529.69,
        },
        A42Entry {
            a_cap: 57,
            b_cap: 2.01,
            c_cap: 83996.85,
        },
        A42Entry {
            a_cap: 56,
            b_cap: 5.24,
            c_cap: 71430.7,
        },
        A42Entry {
            a_cap: 49,
            b_cap: 3.25,
            c_cap: 2544.31,
        },
        A42Entry {
            a_cap: 47,
            b_cap: 2.58,
            c_cap: 775.52,
        },
        A42Entry {
            a_cap: 45,
            b_cap: 5.54,
            c_cap: 9437.76,
        },
        A42Entry {
            a_cap: 43,
            b_cap: 6.01,
            c_cap: 6275.96,
        },
        A42Entry {
            a_cap: 39,
            b_cap: 5.36,
            c_cap: 4694.0,
        },
        A42Entry {
            a_cap: 38,
            b_cap: 2.39,
            c_cap: 8827.39,
        },
        A42Entry {
            a_cap: 37,
            b_cap: 0.83,
            c_cap: 19651.05,
        },
        A42Entry {
            a_cap: 37,
            b_cap: 4.9,
            c_cap: 12139.55,
        },
        A42Entry {
            a_cap: 36,
            b_cap: 1.67,
            c_cap: 12036.46,
        },
        A42Entry {
            a_cap: 35,
            b_cap: 1.84,
            c_cap: 2942.46,
        },
        A42Entry {
            a_cap: 33,
            b_cap: 0.24,
            c_cap: 7084.9,
        },
        A42Entry {
            a_cap: 32,
            b_cap: 0.18,
            c_cap: 5088.63,
        },
        A42Entry {
            a_cap: 32,
            b_cap: 1.78,
            c_cap: 398.15,
        },
        A42Entry {
            a_cap: 28,
            b_cap: 1.21,
            c_cap: 6286.6,
        },
        A42Entry {
            a_cap: 28,
            b_cap: 1.9,
            c_cap: 6279.55,
        },
        A42Entry {
            a_cap: 26,
            b_cap: 4.59,
            c_cap: 10447.39,
        },
    ];

    const A42_R1_ENTRIES_COUNT: usize = 10;
    static A42_R1_ENTRIES: [A42Entry; A42_R1_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 103019,
            b_cap: 1.10749,
            c_cap: 6283.07585,
        },
        A42Entry {
            a_cap: 1721,
            b_cap: 1.0644,
            c_cap: 12566.1517,
        },
        A42Entry {
            a_cap: 702,
            b_cap: 3.142,
            c_cap: 0.0,
        },
        A42Entry {
            a_cap: 32,
            b_cap: 1.02,
            c_cap: 18849.23,
        },
        A42Entry {
            a_cap: 31,
            b_cap: 2.84,
            c_cap: 5507.55,
        },
        A42Entry {
            a_cap: 25,
            b_cap: 1.32,
            c_cap: 5223.69,
        },
        A42Entry {
            a_cap: 18,
            b_cap: 1.42,
            c_cap: 1577.34,
        },
        A42Entry {
            a_cap: 10,
            b_cap: 5.91,
            c_cap: 10977.08,
        },
        A42Entry {
            a_cap: 9,
            b_cap: 1.42,
            c_cap: 6275.96,
        },
        A42Entry {
            a_cap: 9,
            b_cap: 0.27,
            c_cap: 5486.78,
        },
    ];

    const A42_R2_ENTRIES_COUNT: usize = 6;
    static A42_R2_ENTRIES: [A42Entry; A42_R2_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 4359,
            b_cap: 5.7846,
            c_cap: 6283.0758,
        },
        A42Entry {
            a_cap: 124,
            b_cap: 5.579,
            c_cap: 12566.152,
        },
        A42Entry {
            a_cap: 12,
            b_cap: 3.14,
            c_cap: 0.0,
        },
        A42Entry {
            a_cap: 9,
            b_cap: 3.63,
            c_cap: 77713.77,
        },
        A42Entry {
            a_cap: 6,
            b_cap: 1.87,
            c_cap: 5573.14,
        },
        A42Entry {
            a_cap: 3,
            b_cap: 5.47,
            c_cap: 18849.23,
        },
    ];

    const A42_R3_ENTRIES_COUNT: usize = 2;
    static A42_R3_ENTRIES: [A42Entry; A42_R3_ENTRIES_COUNT] = [
        A42Entry {
            a_cap: 145,
            b_cap: 4.273,
            c_cap: 6283.076,
        },
        A42Entry {
            a_cap: 7,
            b_cap: 3.92,
            c_cap: 12566.15,
        },
    ];

    const A42_R4_ENTRIES_COUNT: usize = 1;
    static A42_R4_ENTRIES: [A42Entry; A42_R4_ENTRIES_COUNT] = [A42Entry {
        a_cap: 4,
        b_cap: 2.56,
        c_cap: 6283.08,
    }];

    // Xn entries, for eq. 15 - 19
    #[derive(Debug)]
    struct XIEntry {
        constant: f64,
        mul_pow1: f64,
        mul_pow2: f64,
        div_pow3: i32,
    }
    const XI_ENTRIES_COUNT: usize = 5;
    static XI_ENTRIES: [XIEntry; XI_ENTRIES_COUNT] = [
        XIEntry {
            constant: 297.85036,
            mul_pow1: 445267.111480,
            mul_pow2: -0.0019142,
            div_pow3: 189474,
        },
        XIEntry {
            constant: 357.52772,
            mul_pow1: 35999.050340,
            mul_pow2: -0.0001603,
            div_pow3: -300000,
        },
        XIEntry {
            constant: 134.96298,
            mul_pow1: 477198.867398,
            mul_pow2: 0.0086972,
            div_pow3: 56250,
        },
        XIEntry {
            constant: 93.27191,
            mul_pow1: 483202.017538,
            mul_pow2: -0.0036825,
            div_pow3: 327270,
        },
        XIEntry {
            constant: 125.04452,
            mul_pow1: -1934.136261,
            mul_pow2: 0.0020708,
            div_pow3: 450000,
        },
    ];

    // table A4.3, page 35, A-13
    #[derive(Debug)]
    struct A43Entry {
        y_vec: [i8; 5],

        // psi
        a: i32,
        b: f64,

        // epsilon
        c: i32,
        d: f64,
    }
    const A43_ENTRIES_COUNT: usize = 63;
    static A43_ENTRIES: [A43Entry; A43_ENTRIES_COUNT] = [
        A43Entry {
            y_vec: [0, 0, 0, 0, 1],
            a: -171996,
            b: -174.2,
            c: 92025,
            d: 8.9,
        },
        A43Entry {
            y_vec: [-2, 0, 0, 2, 2],
            a: -13187,
            b: -1.6,
            c: 5736,
            d: -3.,
        },
        A43Entry {
            y_vec: [0, 0, 0, 2, 2],
            a: -2274,
            b: -0.2,
            c: 977,
            d: -0.,
        },
        A43Entry {
            y_vec: [0, 0, 0, 0, 2],
            a: 2062,
            b: 0.2,
            c: -895,
            d: 0.5,
        },
        A43Entry {
            y_vec: [0, 1, 0, 0, 0],
            a: 1426,
            b: -3.4,
            c: 54,
            d: -0.,
        },
        A43Entry {
            y_vec: [0, 0, 1, 0, 0],
            a: 712,
            b: 0.1,
            c: -7,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 1, 0, 2, 2],
            a: -517,
            b: 1.2,
            c: 224,
            d: -0.,
        },
        A43Entry {
            y_vec: [0, 0, 0, 2, 1],
            a: -386,
            b: -0.4,
            c: 200,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 1, 2, 2],
            a: -301,
            b: 0.0,
            c: 129,
            d: -0.,
        },
        A43Entry {
            y_vec: [-2, -1, 0, 2, 2],
            a: 217,
            b: -0.5,
            c: -95,
            d: 0.3,
        },
        A43Entry {
            y_vec: [-2, 0, 1, 0, 0],
            a: -158,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 0, 0, 2, 1],
            a: 129,
            b: 0.1,
            c: -70,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, -1, 2, 2],
            a: 123,
            b: 0.0,
            c: -53,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, 0, 0, 0],
            a: 63,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 1, 0, 1],
            a: 63,
            b: 0.1,
            c: -33,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, -1, 2, 2],
            a: -59,
            b: 0.0,
            c: 26,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, -1, 0, 1],
            a: -58,
            b: -0.1,
            c: 32,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 1, 2, 1],
            a: -51,
            b: 0.0,
            c: 27,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 0, 2, 0, 0],
            a: 48,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, -2, 2, 1],
            a: 46,
            b: 0.0,
            c: -24,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, 0, 2, 2],
            a: -38,
            b: 0.0,
            c: 16,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 2, 2, 2],
            a: -31,
            b: 0.0,
            c: 13,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 2, 0, 0],
            a: 29,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 0, 1, 2, 2],
            a: 29,
            b: 0.0,
            c: -12,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 0, 2, 0],
            a: 26,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 0, 0, 2, 0],
            a: -22,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, -1, 2, 1],
            a: 21,
            b: 0.0,
            c: -10,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 2, 0, 0, 0],
            a: 17,
            b: -0.1,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, -1, 0, 1],
            a: 16,
            b: 0.0,
            c: -8,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 2, 0, 2, 2],
            a: -16,
            b: 0.1,
            c: 7,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 1, 0, 0, 1],
            a: -15,
            b: 0.0,
            c: 9,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 0, 1, 0, 1],
            a: -13,
            b: 0.0,
            c: 7,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, -1, 0, 0, 1],
            a: -12,
            b: 0.0,
            c: 6,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 2, -2, 0],
            a: 11,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, -1, 2, 1],
            a: -10,
            b: 0.0,
            c: 5,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, 1, 2, 2],
            a: -8,
            b: 0.0,
            c: 3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 1, 0, 2, 2],
            a: 7,
            b: 0.0,
            c: -3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 1, 1, 0, 0],
            a: -7,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, -1, 0, 2, 2],
            a: -7,
            b: 0.0,
            c: 3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, 0, 2, 1],
            a: -7,
            b: 0.0,
            c: 3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, 1, 0, 0],
            a: 6,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 0, 2, 2, 2],
            a: 6,
            b: 0.0,
            c: -3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 0, 1, 2, 1],
            a: 6,
            b: 0.0,
            c: -3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, -2, 0, 1],
            a: -6,
            b: 0.0,
            c: 3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, 0, 0, 0, 1],
            a: -6,
            b: 0.0,
            c: 3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, -1, 1, 0, 0],
            a: 5,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, -1, 0, 2, 1],
            a: -5,
            b: 0.0,
            c: 3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 0, 0, 0, 1],
            a: -5,
            b: 0.0,
            c: 3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 2, 2, 1],
            a: -5,
            b: 0.0,
            c: 3,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 0, 2, 0, 1],
            a: 4,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 1, 0, 2, 1],
            a: 4,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 1, -2, 0],
            a: 4,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-1, 0, 1, 0, 0],
            a: -4,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-2, 1, 0, 0, 0],
            a: -4,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [1, 0, 0, 0, 0],
            a: -4,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 1, 2, 0],
            a: 3,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, -2, 2, 2],
            a: -3,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [-1, -1, 1, 0, 0],
            a: -3,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 1, 1, 0, 0],
            a: -3,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, -1, 1, 2, 2],
            a: -3,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, -1, -1, 2, 2],
            a: -3,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [0, 0, 3, 2, 2],
            a: -3,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
        A43Entry {
            y_vec: [2, -1, 0, 2, 2],
            a: -3,
            b: 0.0,
            c: 0,
            d: 0.0,
        },
    ];
}
