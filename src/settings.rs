use std::collections::hash_map::HashMap;

use crate::LOG_DRAIN;

use serde::{Deserialize, Serialize};
use slog::info;

// Describe the settings your policy expects when
// loaded by the policy server.
#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub(crate) struct Settings {
    pub repos: HashMap<String, String>,
}

impl kubewarden::settings::Validatable for Settings {
    fn validate(&self) -> Result<(), String> {
        info!(LOG_DRAIN, "starting settings validation");
        if self.repos.is_empty() {
            info!(LOG_DRAIN, "mapping of repos is empty, skipping");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use kubewarden_policy_sdk::settings::Validatable;

    #[test]
    fn validate_settings() -> Result<(), ()> {
        let settings = Settings {
            repos: HashMap::new(),
        };
        assert!(settings.validate().is_ok());
        Ok(())
    }
}
