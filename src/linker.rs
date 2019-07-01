use crate::errors::*;
use fs_extra;
use std::fs;
use std::path::Path;

#[cfg(windows)]
use tempfile;

pub struct Linker;

// TODO: create a static CopyOptions instance with overwrite = true

impl Linker {
    #[cfg(windows)]
    pub fn check_reparse_privilege() -> Result<()> {
        let src = tempfile::tempdir()?.into_path().join("src");
        let dest = tempfile::tempdir()?.into_path();
        Linker::symlink(&src, &dest).chain_err(|| ErrorKind::FailedToLink(src, dest))
    }

    /// Create a symbolic link from `from` to `to`. `from` must not exist, and
    /// `to` must exist.
    pub fn symlink(from: &Path, to: &Path) -> Result<()> {
        if !Path::exists(to) {
            bail!(ErrorKind::DestinationDoesNotExist(to.to_path_buf()));
        }

        // I can't just convert io::ErrorKind::AlreadyExists in to ErrorKind::SourceExists
        // because on Windows when src is a dir and dest is a file it returns
        // ErrorKind::PermissionDenied.
        if let Err(e) = Linker::os_symlink(from, to) {
            if let Ok(md) = std::fs::symlink_metadata(from) {
                if md.file_type().is_symlink() {
                    if let Ok(target) = std::fs::read_link(from) {
                        if target == to {
                            return Ok(());
                        }
                        bail!(ErrorKind::AlreadyLinked(target));
                    }
                }
                if md.is_dir() || md.is_file() {
                    bail!(ErrorKind::SourceExists(from.to_path_buf()));
                }
            }
            bail!(e);
        }

        Ok(())
    }

    /// This results in a call to CreateSymbolicLinkW
    #[cfg(windows)]
    fn os_symlink(from: &Path, to: &Path) -> std::io::Result<()> {
        if to.is_file() {
            return std::os::windows::fs::symlink_file(to, from);
        }

        std::os::windows::fs::symlink_dir(to, from)
    }

    #[cfg(unix)]
    fn os_symlink(from: &Path, to: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(to, from)
    }

    fn move_item(src: &Path, dest: &Path) -> Result<u64> {
        // fs_extra doesn't attempt to rename files when possible:
        // https://github.com/webdesus/fs_extra/issues/20
        if fs::rename(src, dest).is_ok() {
            return Ok(0);
        }

        if src.is_dir() {
            let mut options = fs_extra::dir::CopyOptions::new();
            options.copy_inside = true;
            return fs_extra::dir::move_dir(src, dest, &options)
                .chain_err(|| ErrorKind::FailedToMove(src.to_path_buf(), dest.to_path_buf()));
        }

        let options = fs_extra::file::CopyOptions::new();
        fs_extra::file::move_file(src, dest, &options)
            .chain_err(|| ErrorKind::FailedToMove(src.to_path_buf(), dest.to_path_buf()))
    }

    /// Attempts to move `src` to `dest`, then create a link from `src` to
    /// `dest`.
    pub fn move_and_link(src: &Path, dest: &Path) -> Result<()> {
        if !src.exists() {
            bail!(ErrorKind::SourceNotFound(src.to_path_buf()));
        }

        if dest.exists() {
            if let Ok(target) = fs::read_link(src) {
                if dest == target {
                    return Ok(());
                }

                bail!(ErrorKind::AlreadyLinked(target));
            }

            bail!(ErrorKind::DestinationExists(dest.to_path_buf()));
        }

        Linker::move_item(src, dest)?;
        Linker::symlink(src, dest)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;
    use tempfile::{tempdir, tempdir_in, NamedTempFile};

    #[test]
    fn symlink_src_none_dest_none() {
        let c = tempdir().unwrap();
        let src = c.path().join("src");
        let dest = c.path().join("dest");
        let err = Linker::symlink(&src, &dest).unwrap_err();

        assert!(match err.kind() {
            ErrorKind::DestinationDoesNotExist(_) => true,
            _ => false,
        });
    }

    #[test]
    fn symlink_src_none_dest_dir() {
        let c = tempdir().unwrap();
        let src = c.path().join("src");
        let dest = tempdir().unwrap().into_path();
        Linker::symlink(&src, &dest).unwrap();
    }

    #[test]
    fn symlink_src_none_dest_dir_twice() {
        let c = tempdir().unwrap();
        let src = c.path().join("src");
        let dest = tempdir().unwrap().into_path();
        Linker::symlink(&src, &dest).unwrap();
        Linker::symlink(&src, &dest).unwrap();
    }

    #[test]
    fn symlink_src_none_dest_file() {
        let c = tempdir().unwrap();
        let src = c.path().join("src");
        let dest = NamedTempFile::new().unwrap().into_temp_path();
        Linker::symlink(&src, &dest).unwrap();
    }

    #[test]
    fn symlink_src_none_dest_file_twice() {
        let c = tempdir().unwrap();
        let src = c.path().join("src");
        let dest = NamedTempFile::new().unwrap().into_temp_path();
        Linker::symlink(&src, &dest).unwrap();
        Linker::symlink(&src, &dest).unwrap();
    }

    #[test]
    fn symlink_src_file_dest_file() {
        let src = NamedTempFile::new().unwrap().into_temp_path();
        let dest = NamedTempFile::new().unwrap().into_temp_path();
        let err = Linker::symlink(&src, &dest).unwrap_err();

        assert!(match err.kind() {
            ErrorKind::SourceExists(_) => true,
            _ => false,
        });
    }

    #[test]
    fn symlink_src_file_dest_dir() {
        let src = NamedTempFile::new().unwrap().into_temp_path();
        let dest = tempdir().unwrap().into_path();
        let err = Linker::symlink(&src, &dest).unwrap_err();

        assert!(match err.kind() {
            ErrorKind::SourceExists(_) => true,
            _ => false,
        });
    }

    #[test]
    fn symlink_src_dir_dest_file() {
        let src = tempdir().unwrap().into_path();
        let dest = NamedTempFile::new().unwrap().into_temp_path();
        let err = Linker::symlink(&src, &dest).unwrap_err();

        assert!(match err.kind() {
            ErrorKind::SourceExists(_) => true,
            _ => false,
        });
    }

    #[test]
    fn symlink_src_dir_dest_dir() {
        let src = tempdir().unwrap().into_path();
        let dest = tempdir().unwrap().into_path();
        let err = Linker::symlink(&src, &dest).unwrap_err();

        assert!(match err.kind() {
            ErrorKind::SourceExists(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_move_and_link_none() {
        let src = tempdir().unwrap().into_path().join("src");
        let err = Linker::move_and_link(&src, &src).unwrap_err();
        assert!(match err.kind() {
            ErrorKind::SourceNotFound(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_move_and_link_dir() {
        let container = tempdir().unwrap();

        let src = tempdir_in(&container).unwrap().into_path();
        assert!(Path::exists(&src));

        let dest = tempdir_in(&container).unwrap().into_path().join("dest");
        assert!(!Path::exists(&dest));

        Linker::move_and_link(&src, &dest).unwrap();
        assert_eq!(fs::read_link(src).unwrap(), dest);
    }

    #[test]
    fn test_move_and_link_dir_to_itself() {
        let src = tempdir().unwrap().into_path();
        let err = Linker::move_and_link(&src, &src).unwrap_err();
        assert!(match err.kind() {
            ErrorKind::DestinationExists(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_move_and_link_file_to_itself() {
        let src = NamedTempFile::new().unwrap().into_temp_path();
        let err = Linker::move_and_link(&src, &src).unwrap_err();
        assert!(match err.kind() {
            ErrorKind::DestinationExists(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_move_and_link_dir_which_is_already_correctly_linked() {
        let container = tempdir().unwrap().into_path();

        let src = container.join("src");
        fs::create_dir(&src).unwrap();
        assert!(Path::exists(&src));

        let dest = container.join("dest");
        assert!(!Path::exists(&dest));

        Linker::move_and_link(&src, &dest).unwrap();
        assert_eq!(fs::read_link(&src).unwrap(), dest);

        Linker::move_and_link(&src, &dest).unwrap();
        assert_eq!(fs::read_link(&src).unwrap(), dest);
    }

    #[test]
    fn test_move_and_link_dir_to_existing_dir() {
        let container = tempdir().unwrap();

        let src = tempdir_in(&container).unwrap().into_path();
        assert!(Path::exists(&src));

        let dest = tempdir_in(&container).unwrap().into_path();
        assert!(Path::exists(&dest));

        let e = Linker::move_and_link(&src, &dest).unwrap_err();
        assert!(match e.kind() {
            ErrorKind::DestinationExists(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_move_and_link_dir_to_existing_file() {
        let container = tempdir().unwrap();

        let src = &tempdir_in(&container).unwrap().into_path();
        assert!(Path::exists(src));

        let dest = &NamedTempFile::new().unwrap().into_temp_path();
        assert!(Path::exists(dest));

        let e = Linker::move_and_link(src, dest).unwrap_err();
        assert!(match e.kind() {
            ErrorKind::DestinationExists(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_move_and_link_file() {
        let src = NamedTempFile::new().unwrap().into_temp_path();
        assert!(Path::exists(&src));

        let dest = tempdir().unwrap().into_path().join("to");
        assert!(!Path::exists(&dest));

        Linker::move_and_link(&src, &dest).unwrap();
        assert_eq!(fs::read_link(&src).unwrap(), dest.to_path_buf());
    }

    #[test]
    fn test_move_and_link_file_which_is_already_correctly_linked() {
        let src = NamedTempFile::new().unwrap().into_temp_path();
        assert!(Path::exists(&src));

        let dest = tempdir().unwrap().into_path().join("to");
        assert!(!Path::exists(&dest));

        Linker::move_and_link(&src, &dest).unwrap();
        assert_eq!(fs::read_link(&src).unwrap(), dest.to_path_buf());

        Linker::move_and_link(&src, &dest).unwrap();
        assert_eq!(fs::read_link(&src).unwrap(), dest.to_path_buf());
    }

    #[test]
    fn test_move_and_link_file_to_existing_dir() {
        let src = NamedTempFile::new().unwrap().into_temp_path();
        assert!(Path::exists(&src));

        let dest = tempdir().unwrap().into_path();
        assert!(Path::exists(&dest));

        let e = Linker::move_and_link(&src, &dest).unwrap_err();
        assert!(match e.kind() {
            ErrorKind::DestinationExists(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_move_and_link_file_to_existing_file() {
        let src = NamedTempFile::new().unwrap().into_temp_path();
        assert!(Path::exists(&src));

        let dest = NamedTempFile::new().unwrap().into_temp_path();
        assert!(Path::exists(&dest));

        let e = Linker::move_and_link(&src, &dest).unwrap_err();
        assert!(match e.kind() {
            ErrorKind::DestinationExists(_) => true,
            _ => false,
        });
    }

    #[test]
    #[ignore]
    fn test_move_and_link_large_directory_same_partition() {
        assert!(false);
    }

    #[test]
    #[ignore]
    fn test_move_and_link_large_directory_other_partition() {
        std::env::var("SAVELI_TEST_OTHER_PARTITION_DESTINATION").unwrap();
    }

    #[test]
    #[ignore]
    fn test_move_and_link_large_directory_other_disk() {
        std::env::var("SAVELI_TEST_OTHER_DISK_DESTINATION").unwrap();
    }
}
