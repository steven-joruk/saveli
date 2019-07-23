## Saveli

[![Build Status](https://travis-ci.com/steven-joruk/saveli.svg?branch=master)](https://travis-ci.com/steven-joruk/saveli)

Manage links to game saves and other data.

```
Saveli 0.1.0
Steven Joruk <steven@joruk.com>
Moves game saves and creates links in their place.

USAGE:
    saveli.exe [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -v, --version    Prints version information

SUBCOMMANDS:
    heed                The inverse of ignore
    ignore              Ignore a game entry by id, preventing it from being linked, restored or unlinked
    link                Move game saves from their original locations to the storage path and create links to their
                        new location
    restore             Creates links to game saves which have been moved to the storage path
    search              Search the database for the keyword
    set-storage-path    Set where game saves and meta data should be stored.
    unlink              The inverse of link
```

## FAQ

### Windows - Is running as administrator really necessary?

Unfortunately yes, creating junction points requires administrator privileges. It's likely to change in a future update to Windows 10.
