use crate::game::Game;
use crate::errors::*;
use serde::Deserialize;
use std::path::Path;
use std::io::Write;

const VERSION: usize = 1;

#[derive(Deserialize, Debug)]
pub struct Database {
    version: usize,
    pub games: Vec<Game>,
}

impl Database {
    pub fn new<T: AsRef<Path>>(storage_path: T) -> Result<Database> {
        let windows_path = storage_path.as_ref().join("windows.json");
        if !windows_path.exists() {
            println!("Creating {}", windows_path.display());
            let mut f = std::fs::File::create(&windows_path)?;
            f.write_all(include_bytes!("../res/windows.json"))?;
        }

        Database::load_from(&windows_path)
    }

    fn load_from<T: AsRef<Path>>(path: T) -> Result<Database> {
        let data = std::fs::read_to_string(&path)?;
        let db = Database::load(data)?;
        println!("Loaded {} game entries from {}", db.games.len(), path.as_ref().display());
        Ok(db)
    }

    fn load<T: AsRef<str>>(data: T) -> Result<Database> {
        let db: Database = serde_json::from_str(data.as_ref())?;

        if db.version > VERSION {
            bail!(ErrorKind::DatabaseTooNew(db.version, VERSION));
        }

        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_load_older_version_succeeds() {
        let json = json!({ "version": VERSION - 1, "games": [] });
        Database::load(json.to_string()).unwrap();
    }

    #[test]
    fn test_load_current_version_succeeds() {
        let json = json!({ "version": VERSION, "games": [] });
        Database::load(json.to_string()).unwrap();
    }

    #[test]
    fn test_load_newer_version_fails() {
        let json = json!({ "version": VERSION + 1, "games": [] });
        Database::load(json.to_string()).unwrap_err();
    }
}