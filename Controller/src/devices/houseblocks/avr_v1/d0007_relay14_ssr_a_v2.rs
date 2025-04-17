pub mod logic {
    use super::{super::common::relay14_common_a::logic, hardware};

    #[derive(Debug)]
    pub struct Specification {}
    impl logic::Specification for Specification {
        type HardwareSpecification = hardware::Specification;

        fn class() -> &'static str {
            "relay14_ssr_a_v2"
        }
    }

    pub type DeviceFactory = logic::DeviceFactory<Specification>;
    pub type Device<'h> = logic::Device<'h, Specification>;
    pub type SignalIdentifier = logic::SignalIdentifier;
}
pub mod hardware {
    pub use super::super::common::relay14_common_a::hardware::{OUTPUT_COUNT, PropertiesRemote};
    use super::super::{
        super::houseblocks_v1::common::AddressDeviceType, common::relay14_common_a::hardware,
    };

    #[derive(Debug)]
    pub struct Specification {}
    impl hardware::Specification for Specification {
        fn device_type_name() -> &'static str {
            "Relay14_SSR_A_v2"
        }
        fn address_device_type() -> AddressDeviceType {
            AddressDeviceType::new_from_ordinal(7).unwrap()
        }
    }

    pub type Device = hardware::Device<Specification>;
}
