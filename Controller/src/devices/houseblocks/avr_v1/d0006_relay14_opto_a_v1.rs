pub mod logic {
    use super::{super::common::relay14_common_a::logic, hardware};

    #[derive(Debug)]
    pub struct Specification {}
    impl logic::Specification for Specification {
        type HardwareSpecification = hardware::Specification;

        fn class() -> &'static str {
            "relay14_opto_a_v1"
        }
    }

    pub type Device = logic::Device<Specification>;
}
pub mod hardware {
    pub use super::super::common::relay14_common_a::hardware::{PropertiesRemote, OUTPUT_COUNT};
    use super::super::{
        super::houseblocks_v1::common::AddressDeviceType, common::relay14_common_a::hardware,
    };

    #[derive(Debug)]
    pub struct Specification {}
    impl hardware::Specification for Specification {
        fn device_type_name() -> &'static str {
            "Relay14_Opto_A_v1"
        }
        fn address_device_type() -> AddressDeviceType {
            AddressDeviceType::new_from_ordinal(6).unwrap()
        }
    }

    pub type Device = hardware::Device<Specification>;
}
