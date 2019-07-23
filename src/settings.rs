use crate::errors::*;
use crate::game::Game;
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
    #[serde(default)]
    ignored: Vec<String>,
}

impl Settings {
    fn get_settings_path() -> Result<PathBuf> {
        let path = app_dirs::app_root(AppDataType::UserConfig, &APP_INFO)?;
        Ok(path.join("settings.json"))
    }

    pub fn save(&self) -> Result<()> {
        let path = Settings::get_settings_path()?;
        println!("Saving settings to {}", path.display());
        let file = std::fs::File::create(&path)?;
        Ok(serde_json::to_writer_pretty(&file, self)?)
    }

    pub fn load() -> Result<Settings> {
        let path = Settings::get_settings_path()?;
        let data = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn ignore_game(&mut self, game: &Game) -> Result<()> {
        println!("Ignoring {}", game.title);
        self.ignored.push(game.id.clone());
        self.save()
    }

    pub fn heed_game(&mut self, game: &Game) -> Result<()> {
        println!("Heeding {}", game.title);
        self.ignored.retain(|id| *id != game.id);
        self.save()
    }

    pub fn game_is_ignored(&self, id: &String) -> bool {
        return self.ignored.contains(id);
    }
}
