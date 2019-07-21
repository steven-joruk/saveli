use crate::errors::*;
use app_dirs::{AppDataType, AppInfo};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// TODO: Get from Cargo.toml
const APP_INFO: AppInfo = AppInfo {
    name: "saveli",
    author: "saveli-project",
};

#[derive(Default, Deserialize, Serialize)]
pub struct Settings {
    pub storage_path: PathBuf,
    #[serde(skip)]
    pub dry_run: bool,
}

impl Settings {
    fn get_settings_path() -> Result<PathBuf> {
        let path = app_dirs::app_root(AppDataType::UserConfig, &APP_INFO)?;
        Ok(path.join("settings.json"))
    }

    pub fn save(&self) -> Result<()> {
        let path = Settings::get_settings_path()?;
        let file = std::fs::File::create(&path)?;
        Ok(serde_json::to_writer_pretty(&file, self)?)
    }

    pub fn load() -> Settings {
        if let Ok(path) = Settings::get_settings_path() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str(&data) {
                    return settings;
                }
            }
        }

        Settings::default()
    }
}
