use std::path::PathBuf;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    errors {
        DatabaseTooNew(version: usize, supported: usize) {
            display("The database version ({}) is too new, up to version {} is supported", version, supported)
        }

        DestinationDoesNotExist(path: PathBuf) {
            display("No file or directory exists at {}", path.display())
        }

        SourceExists(path: PathBuf) {
            display("A file or directory already exists at {}", path.display())
        }

        AlreadyLinked(target: PathBuf) {
            display("The source is already a link to: {}", target.display())
        }

        FailedToMove(from: PathBuf, to: PathBuf) {
            display("Failed to move {} to {}", from.display(), to.display())
        }
    }

    foreign_links {
        AppDirs(app_dirs::AppDirsError);
        FsExtra(fs_extra::error::Error);
        Io(std::io::Error);
        Json(serde_json::error::Error);
    }
}
