use crate::errors::*;
use crate::game::Game;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const VERSION: usize = 1;

#[derive(Deserialize, Debug, Serialize)]
pub struct Database {
    version: usize,
    pub games: Vec<Game>,
    #[serde(skip)]
    path: PathBuf,
}

impl Database {
    pub fn new<T: AsRef<Path>>(storage_path: T) -> Result<Database> {
        let mut db: Database;

        let windows_path = storage_path.as_ref().join("windows.json");
        if windows_path.exists() {
            db = Database::load_from(&windows_path)?;
        } else {
            db = Database::load(include_str!("../res/windows.json"))?;
            db.path = windows_path;
            db.save()?;
        }

        Ok(db)
    }

    pub fn save(&self) -> Result<()> {
        println!("Saving {}", self.path.display());
        let f = std::fs::File::create(&self.path)?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }

    fn load_from<T: AsRef<Path>>(path: T) -> Result<Database> {
        let data = std::fs::read_to_string(&path)?;
        let mut db = Database::load(data)?;
        db.path = path.as_ref().to_path_buf();
        println!(
            "Loaded {} game entries from {}",
            db.games.len(),
            path.as_ref().display()
        );
        Ok(db)
    }

    fn load<T: AsRef<str>>(data: T) -> Result<Database> {
        let mut db: Database = serde_json::from_str(data.as_ref())?;

        if db.version > VERSION {
            bail!(ErrorKind::DatabaseTooNew(db.version, VERSION));
        }

        // The sorting of Game prioratises customisations.
        db.games.sort();
        db.games.dedup();

        // Convert path variables to expanded paths
        for game in &mut db.games {
            for save in &mut game.saves {
                save.update_path()?;
            }
        }

        Ok(db)
    }

    pub fn search(&self, keyword: &str) {
        if keyword.is_empty() {
            eprintln!("The keyword must not be empty");
            return;
        }

        let mut missed = true;

        for game in &self.games {
            if game.id.contains(keyword) || game.title.contains(keyword) {
                println!("Found {} ({})", game.title, game.id);
                missed = false;
            }
        }

        if missed {
            println!("Couldn't find any matching games");
        }
    }

    pub fn add(&mut self, game: Game) -> Result<()> {
        self.games.retain(|g| !(*g == game && g.custom));
        self.games.push(game);
        self.save()
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
