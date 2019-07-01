use crate::errors::{Error, Result};
use crate::linker::Linker;
use serde::{Deserialize, Deserializer};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SavePath {
    id: String,
    #[serde(deserialize_with = "deserialize_path")]
    path: PathBuf,
}

fn deserialize_path<'de, D>(deserializer: D) -> std::result::Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let orig = String::deserialize(deserializer)?;
    let expanded = shellexpand::env(&orig)
        .map_err(serde::de::Error::custom)?
        .into_owned();

    let path = PathBuf::from(&expanded);
    if path.is_relative() {
        let msg = format!("ignoring relative path: {}", path.display());
        return Err(serde::de::Error::custom(msg));
    }

    Ok(path)
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Game {
    title: String,
    id: String,
    saves: Vec<SavePath>,
}

impl Game {
    /// Returns all games which have existing save paths. This will also return
    /// games which aren't installed.
    pub fn all_with_saves(games: &[Game]) -> Vec<&Game> {
        games.iter().filter(|g| g.has_saves()).collect()
    }

    /// Returns games which have saves in the storage path.
    pub fn all_with_moved_saves<'g>(games: &'g [Game], storage_path: &Path) -> Vec<&'g Game> {
        games
            .iter()
            .filter(|g| !g.id.is_empty())
            .filter(|g| Path::exists(&storage_path.join(&g.id)))
            .collect()
    }

    /// Attempts to move the game's save paths to the storage location and
    /// create corresponding links.
    pub fn move_and_link(&self, storage_path: &Path) -> Result<()> {
        let game_storage_path = storage_path.join(&self.id);
        if let Err(e) = std::fs::create_dir_all(&game_storage_path) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(Error::from(e));
            }
        }

        for s in &self.saves {
            let dest = game_storage_path.join(&s.id);
            println!(
                "Linking {}'s {} to {}",
                self.title,
                s.path.display(),
                dest.display()
            );
            Linker::move_and_link(&s.path, &dest)?
        }

        Ok(())
    }

    /// If saves exist, it will attempt to create links. It will fail if real
    /// files or directories already exist.
    pub fn restore(&self, storage_path: &Path) -> Result<()> {
        for s in &self.saves {
            let dest = storage_path.join(&self.id).join(&s.id);
            println!(
                "Restoring {}'s {} from {}",
                self.title,
                s.path.display(),
                dest.display()
            );
            Linker::symlink(&s.path, &dest)?;
        }

        Ok(())
    }

    fn has_saves(&self) -> bool {
        self.saves.iter().any(|s| Path::exists(&s.path))
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{Game, SavePath};

    #[test]
    fn test_all_with_saves_matches() {
        let mut game = Game::default();
        game.saves = vec![SavePath {
            path: tempfile::tempdir().unwrap().into_path(),
            ..Default::default()
        }];
        assert_eq!(Game::all_with_saves(&vec![game]).len(), 1)
    }

    #[test]
    fn test_all_with_saves_empty() {
        assert_eq!(Game::all_with_saves(&vec![Game::default()]).len(), 0)
    }

    #[test]
    fn test_all_with_moved_saves_matches() {
        let game = Game {
            id: String::from("gameid"),
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
    fn test_move_and_link_file() {
        let src = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        assert!(src.exists());
        let game = Game {
            id: String::from("gameid"),
            saves: vec![SavePath {
                id: String::from("saveid"),
                path: src.to_path_buf(),
            }],
            ..Default::default()
        };
        let storage_path = tempfile::tempdir().unwrap().into_path();
        Game::move_and_link(&game, &storage_path).unwrap();
        let dest = storage_path.join(&game.id).join("saveid");
        assert_eq!(std::fs::read_link(&src).unwrap(), dest);
    }

    #[test]
    fn test_move_and_link_dir() {
        let src = tempfile::tempdir().unwrap().into_path();
        assert!(src.exists());
        let game = Game {
            id: String::from("gameid"),
            saves: vec![SavePath {
                id: String::from("saveid"),
                path: src.to_path_buf(),
            }],
            ..Default::default()
        };
        let storage_path = tempfile::tempdir().unwrap().into_path();
        Game::move_and_link(&game, &storage_path).unwrap();
        let dest = storage_path.join(&game.id).join("saveid");
        assert_eq!(std::fs::read_link(&src).unwrap(), dest);
    }

    #[test]
    fn test_move_and_link_existing_file() {
        let src = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        assert!(src.exists());
        let game = Game {
            id: String::from("gameid"),
            saves: vec![SavePath {
                id: String::from("saveid"),
                path: src.to_path_buf(),
            }],
            ..Default::default()
        };
        let storage_path = tempfile::tempdir().unwrap().into_path();
        let dest = storage_path.join(&game.id).join("saveid");
        std::fs::create_dir_all(&storage_path.join(&game.id)).unwrap();
        std::fs::File::create(&dest).unwrap();
        assert!(Game::move_and_link(&game, &storage_path).is_err());
    }

    #[test]
    fn test_move_and_link_existing_dir() {
        let src = tempfile::tempdir().unwrap().into_path();
        assert!(src.exists());
        let game = Game {
            id: String::from("gameid"),
            saves: vec![SavePath {
                id: String::from("saveid"),
                path: src.to_path_buf(),
            }],
            ..Default::default()
        };
        let storage_path = tempfile::tempdir().unwrap().into_path();
        let dest = storage_path.join(&game.id).join("saveid");
        std::fs::create_dir_all(&dest).unwrap();
        assert!(Game::move_and_link(&game, &storage_path).is_err());
    }

    #[test]
    fn test_restore_dir() {}

    #[test]
    fn test_restore_file() {}

    #[test]
    fn test_restore_existing_dir() {}

    #[test]
    fn test_restore_existing_file() {}
}
