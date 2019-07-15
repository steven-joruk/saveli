use crate::game::Game;
use crate::errors::*;
use serde::Deserialize;

const VERSION: usize = 1;

#[derive(Deserialize, Debug)]
pub struct Database {
    version: usize,
    pub games: Vec<Game>,
}

impl Database {
    pub fn load<T: AsRef<str>>(data: T) -> Result<Database> {
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