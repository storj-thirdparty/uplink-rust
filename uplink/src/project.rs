//! Storj DCS Project.

use crate::access::Grant;
use crate::config::Config;

/// TODO: document it.
pub struct Project {}

impl Project {
    /// TODO: implement & document this method.
    pub fn open_project_with_config(config: &Config, grant: Grant) -> Self {
        // call uplink_config_open_project()
        Self {}
    }

    /// TODO: implement & document this method.
    pub fn revoke_access(&self) {
        todo!("implement it")
    }
}
