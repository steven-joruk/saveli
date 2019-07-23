use crate::database::Database;
use crate::errors::{Error, Result};
use crate::linker::Linker;
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::cmp::{Ord, Ordering, PartialOrd};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SavePath {
    pub id: String,
    path: String,
    // It would be much nicer to be able to set this as part of finalizing the
    // deserialization of the SavePath. Remove update_path if this gets fixed.
    // Watch https://github.com/serde-rs/serde/issues/642
    #[serde(skip)]
    pub expanded: PathBuf,
}

impl SavePath {
    pub fn new<T: AsRef<str>>(id: String, path: T) -> Result<SavePath> {
        let mut save_path = SavePath {
            id,
            ..Default::default()
        };

        save_path.set_path(path)?;
        Ok(save_path)
    }

    pub fn update_path(&mut self) -> Result<()> {
        let path = self.path.to_owned();
        self.set_path(&path)
    }

    pub fn set_path<T: AsRef<str>>(&mut self, path: T) -> Result<()> {
        let trimmed = path.as_ref().trim();
        if !trimmed.starts_with('$') {
            eprintln!("The path doesn't start with a variable: {}", trimmed);
        }

        self.path = trimmed.to_owned();
        let expanded_str = shellexpand::env(&self.path).unwrap_or_default();
        self.expanded = PathBuf::from(expanded_str.into_owned());
        if self.expanded.is_relative() {
            bail!("Found relative path: {}", self.expanded.display());
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Game {
    pub title: String,
    pub id: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub custom: bool,
    pub saves: Vec<SavePath>,
}

fn is_false(v: &bool) -> bool {
    !*v
}

impl PartialOrd for Game {
    fn partial_cmp(&self, other: &Game) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Game {
    fn cmp(&self, other: &Game) -> Ordering {
        if self.id < other.id {
            Ordering::Less
        } else if self.id > other.id {
            Ordering::Greater
        } else if self.custom && !other.custom {
            Ordering::Less
        } else if !self.custom && other.custom {
            Ordering::Greater
        } else {
            self.title.cmp(&other.title)
        }
    }
}

impl Eq for Game {}

impl PartialEq for Game {
    fn eq(&self, other: &Game) -> bool {
        self.id == other.id
    }
}

impl Game {
    pub fn link_all(db: &Database, settings: &Settings) -> Result<()> {
        let movable = Game::all_with_movable_saves(&db.games);
        println!(
            "Found {} games with saves in their standard locations",
            movable.len()
        );

        for game in movable {
            if settings.game_is_ignored(&game.id) {
                println!("{} is ignored, skipping", game.title);
            } else if let Err(e) = game.link(&settings.storage_path, settings.dry_run) {
                eprintln!("{}", e);
            }
        }

        Ok(())
    }

    pub fn restore_all(db: &Database, settings: &Settings) -> Result<()> {
        let restorable = Game::all_with_moved_saves(&db.games, &settings.storage_path);
        println!(
            "Found {} games with saves moved to {}",
            restorable.len(),
            settings.storage_path.display()
        );

        for game in restorable {
            if settings.game_is_ignored(&game.id) {
                println!("{} is ignored, skipping", game.title);
            } else if let Err(e) = game.restore(&settings.storage_path, settings.dry_run) {
                eprintln!("{}", e);
            }
        }

        Ok(())
    }

    pub fn unlink_all(db: &Database, settings: &Settings) -> Result<()> {
        let restorable = Game::all_with_moved_saves(&db.games, &settings.storage_path);
        println!("Found {} games with moved saves", restorable.len());

        for game in restorable {
            if settings.game_is_ignored(&game.id) {
                println!("{} is ignored, skipping", game.title);
            } else if let Err(e) = game.unlink(&settings.storage_path, settings.dry_run) {
                eprintln!("{}", e);
            }
        }

        Ok(())
    }

    fn all_with_movable_saves(games: &[Game]) -> Vec<&Game> {
        games.iter().filter(|g| g.has_movable_saves()).collect()
    }

    /// Returns games which have saves in the storage path.
    fn all_with_moved_saves<'g>(games: &'g [Game], storage_path: &Path) -> Vec<&'g Game> {
        games
            .iter()
            .filter(|g| !g.id.is_empty())
            .filter(|g| Path::exists(&storage_path.join(&g.id)))
            .collect()
    }

    /// Attempts to move the game's save paths to the storage location and
    /// create corresponding links.
    pub fn link(&self, storage_path: &Path, dry_run: bool) -> Result<()> {
        let game_storage_path = storage_path.join(&self.id);
        if !dry_run {
            Linker::verify_reparse_privilege()?;

            if let Err(e) = std::fs::create_dir_all(&game_storage_path) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(Error::from(e));
                }
            }
        }

        for s in &self.saves {
            let dest = game_storage_path.join(&s.id);
            println!(
                "Linking {}'s {} to {}",
                self.title,
                s.expanded.display(),
                dest.display()
            );

            if !dry_run {
                println!("Moving {} to {}", s.expanded.display(), dest.display());
                Linker::move_item(&s.expanded, &dest)?;
                println!(
                    "Creating a link from {} to {}",
                    s.expanded.display(),
                    dest.display()
                );
                Linker::symlink(&s.expanded, &dest)?;
            }
        }

        Ok(())
    }

    /// If saves exist, it will attempt to create links. It will fail if real
    /// files or directories already exist.
    pub fn restore(&self, storage_path: &Path, dry_run: bool) -> Result<()> {
        if !dry_run {
            Linker::verify_reparse_privilege()?;
        }

        for s in &self.saves {
            let dest = storage_path.join(&self.id).join(&s.id);
            println!(
                "Restoring {}'s {} from {}",
                self.title,
                s.expanded.display(),
                dest.display()
            );

            if !dry_run {
                Linker::symlink(&s.expanded, &dest)?;
            }
        }

        Ok(())
    }

    /// The inverse of link.
    pub fn unlink(&self, storage_path: &Path, dry_run: bool) -> Result<()> {
        if !dry_run {
            Linker::verify_reparse_privilege()?;
        }

        for s in &self.saves {
            let dest = storage_path.join(&self.id).join(&s.id);
            // TODO: Check it exists
            println!(
                "Unlinking {}'s {} from {}",
                self.title,
                s.expanded.display(),
                dest.display()
            );

            if !dry_run {
                println!("Removing {}", s.expanded.display());
                std::fs::remove_dir(&s.path)?;
                println!("Moving {} to {}", dest.display(), s.expanded.display());
                Linker::move_item(&dest, &s.expanded)?;
            }
        }

        if !dry_run {
            let game_storage_path = storage_path.join(&self.id);
            println!("Removing {}", game_storage_path.display());
            std::fs::remove_dir(game_storage_path)?;
        }

        Ok(())
    }

    fn has_movable_saves(&self) -> bool {
        self.saves
            .iter()
            .any(|s| match std::fs::symlink_metadata(&s.expanded) {
                Ok(md) => !md.file_type().is_symlink(),
                Err(_) => false,
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{Game, SavePath};

    #[test]
    fn test_all_with_moved_saves_matches() {
        let game = Game {
            id: "gameid".to_owned(),
            ..Default::default()
        };
        let storage_path = tempfile::tempdir().unwrap().into_path();
        std::fs::create_dir(storage_path.join(&game.id)).unwrap();
        assert_eq!(
            Game::all_with_moved_saves(&vec![game], &storage_path).len(),
            1
        )
    }

    #[test]
    fn test_all_with_moved_saves_empty() {
        let storage_path = tempfile::tempdir().unwrap().into_path();
        assert_eq!(
            Game::all_with_moved_saves(&vec![Game::default()], &storage_path).len(),
            0
        )
    }

    #[test]
    fn test_link_file() {
        let src = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        assert!(src.exists());
        let game = Game {
            id: "gameid".to_owned(),
            saves: vec![SavePath::new("saveid".to_owned(), src.to_str().unwrap()).unwrap()],
            ..Default::default()
        };
        let storage_path = tempfile::tempdir().unwrap().into_path();
        Game::link(&game, &storage_path, false).unwrap();
        let dest = storage_path.join(&game.id).join("saveid");
        assert_eq!(std::fs::read_link(&src).unwrap(), dest);
    }

    #[test]
    fn test_move_and_link_dir() {
        let src = tempfile::tempdir().unwrap().into_path();
        assert!(src.exists());
        let game = Game {
            id: "gameid".to_owned(),
            saves: vec![SavePath::new("saveid".to_owned(), src.to_str().unwrap()).unwrap()],
            ..Default::default()
        };
        let storage_path = tempfile::tempdir().unwrap().into_path();
        Game::link(&game, &storage_path, false).unwrap();
        let dest = storage_path.join(&game.id).join("saveid");
        assert_eq!(std::fs::read_link(&src).unwrap(), dest);
    }

    #[test]
    fn test_move_and_link_existing_file() {
        let src = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        assert!(src.exists());
        let game = Game {
            id: "gameid".to_owned(),
            saves: vec![SavePath::new("saveid".to_owned(), src.to_str().unwrap()).unwrap()],
            ..Default::default()
        };
        let storage_path = tempfile::tempdir().unwrap().into_path();
        let dest = storage_path.join(&game.id).join("saveid");
        std::fs::create_dir_all(&storage_path.join(&game.id)).unwrap();
        std::fs::File::create(&dest).unwrap();
        Game::link(&game, &storage_path, false).unwrap();
    }

    #[test]
    fn test_move_and_link_existing_dir() {
        let src = tempfile::tempdir().unwrap().into_path();
        assert!(src.exists());
        let game = Game {
            id: "gameid".to_owned(),
            saves: vec![SavePath::new("saveid".to_owned(), src.to_str().unwrap()).unwrap()],
            ..Default::default()
        };
        let storage_path = tempfile::tempdir().unwrap().into_path();
        let dest = storage_path.join(&game.id).join("saveid");
        std::fs::create_dir_all(&dest).unwrap();
        Game::link(&game, &storage_path, false).unwrap();
    }

    #[test]
    fn test_restore_dir() {}

    #[test]
    fn test_restore_file() {}

    #[test]
    fn test_restore_existing_dir() {}

    #[test]
    fn test_restore_existing_file() {}

    // TODO: Unlink tests
}
