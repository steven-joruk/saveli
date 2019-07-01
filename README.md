## Saveli

[![Build Status](https://travis-ci.com/steven-joruk/saveli.svg?branch=master)](https://travis-ci.com/steven-joruk/saveli)

Manage links to game saves and other data.

```
Moves game saves and creates links in their place.

USAGE:
    saveli.exe <storage-path> <--link|--restore>

FLAGS:
    -h, --help       Prints help information
    -l, --link       Move game saves from their original locations to the storage path and create links to their new
                     location.
    -r, --restore    Creates links to game saves which have been moved to the storage path.
    -v, --version    Prints version information

ARGS:
    <storage-path>    The location game saves and meta data should be stored.
```

## FAQ

### Windows - Is running as administrator really necessary?

Unfortunately yes, creating junction points requires administrator privileges. It's likely to change in a future update to Windows 10.
