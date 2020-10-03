use super::DeviceIdSignalId;
use std::collections::{HashMap, HashSet};

pub type Connections = HashMap<DeviceIdSignalId, HashSet<DeviceIdSignalId>>;
