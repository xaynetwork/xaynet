use std::path::PathBuf;

use crate::settings::Settings;
use crate::settings::SettingsError;

pub fn init(config_path: &PathBuf) -> Result<Settings, SettingsError> {
    tracing::debug!("initialize");
    Settings::new(config_path)
}
