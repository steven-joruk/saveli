#[macro_use]
extern crate error_chain;

mod database;
mod errors;
mod game;
mod linker;

use clap::{App, AppSettings, Arg, ArgGroup, ArgMatches};
use database::Database;
use game::Game;
use linker::Linker;
use std::path::PathBuf;

fn get_options() -> ArgMatches<'static> {
    App::new("Saveli")
        .version(env!("CARGO_PKG_VERSION"))
        .version_short("v")
        .author("Steven Joruk <steven@joruk.com>")
        .about("Moves game saves and creates links in their place.")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::DisableHelpSubcommand)
        .arg(
            Arg::with_name("link")
                .short("l")
                .long("link")
                .help(
                    "Move game saves from their original locations to the \
                     storage path and create links to their new location."
                )
        )
        .arg(
            Arg::with_name("restore")
                .short("r")
                .long("--restore")
                .help(
                    "Creates links to game saves which have been moved to the \
                     storage path."
                )
        )
        .group(
            ArgGroup::with_name("command")
                .args(&["link", "restore"])
                .required(true)
        )
        .arg(
            Arg::with_name("storage-path")
                .index(1)
                .required(true)
                .help("The location game saves and meta data should be stored.")
        )
        .get_matches()
}

fn main() {
    if Linker::check_reparse_privilege().is_err() {
        eprintln!(
            "You don't have the required privileges to create links. Try running as administrator."
        );

        return;
    }

    let matches = get_options();
    let storage_path = PathBuf::from(matches.value_of("storage-path").unwrap());

    let db = Database::new(&storage_path).unwrap_or_else(|err| {
        panic!("Failed to parse windows database: {}", err);
    });

    if matches.is_present("link") {
        let movable = Game::all_with_saves(&db.games);
        println!(
            "Found {} games with saves in their standard locations",
            movable.len()
        );

        for game in movable {
            if let Err(e) = game.move_and_link(&storage_path) {
                eprintln!("{}", e);
            }
        }

        return;
    }

    if matches.is_present("restore") {
        let restorable = Game::all_with_moved_saves(&db.games, &storage_path);
        println!(
            "Found {} games with saves moved to {}",
            restorable.len(),
            storage_path.display()
        );

        for game in restorable {
            if let Err(e) = game.restore(&storage_path) {
                eprintln!("{}", e);
            }
        }

        return;
    }
}
