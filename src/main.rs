#![recursion_limit = "128"]
#[macro_use]
extern crate error_chain;

mod database;
mod errors;
mod game;
mod linker;
mod settings;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use database::Database;
use errors::*;
use game::Game;
use settings::Settings;
use std::path::Path;

fn get_command_line_matches() -> ArgMatches<'static> {
    App::new("Saveli")
        .version(env!("CARGO_PKG_VERSION"))
        .version_short("v")
        .author("Steven Joruk <steven@joruk.com>")
        .about("Moves game saves and creates links in their place.")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::DisableHelpSubcommand)
        .subcommand(
            SubCommand::with_name("set-storage-path")
                .about("Set where game saves and meta data should be stored")
                .arg(Arg::with_name("path").index(1).required(true)),
        )
        .subcommand(
            SubCommand::with_name("link")
                .about(
                    "Move game saves from their original locations to the \
                     storage path and create links to their new location",
                )
                .arg(Arg::with_name("dry-run").short("d").long("dry-run")),
        )
        .subcommand(
            SubCommand::with_name("restore")
                .about(
                    "Creates links to game saves which have been moved to the \
                     storage path",
                )
                .arg(Arg::with_name("dry-run").short("d").long("dry-run")),
        )
        .subcommand(
            SubCommand::with_name("unlink")
                .about("The inverse of link")
                .arg(Arg::with_name("dry-run").short("d").long("dry-run")),
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Search the database for the keyword")
                .arg(Arg::with_name("keyword").index(1).required(true)),
        )
        .subcommand(
            SubCommand::with_name("ignore")
                .about(
                    "Ignore a game entry by id, preventing it from being \
                     linked, restored or unlinked",
                )
                .arg(Arg::with_name("id").index(1).required(true)),
        )
        .subcommand(
            SubCommand::with_name("heed")
                .about("The inverse of ignore")
                .arg(Arg::with_name("id").index(1).required(true)),
        )
        .get_matches()
}

fn set_storage_path(path: &Path, settings: &mut Settings) -> Result<()> {
    if path.components().next() == None {
        bail!("You must specify a path");
    }

    settings.storage_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    if let Err(e) = std::fs::create_dir_all(&settings.storage_path) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            bail!(e);
        }
    }

    settings.save()?;

    println!(
        "Your storage path has been set to {}",
        settings.storage_path.display()
    );

    Ok(())
}

fn run() -> Result<()> {
    let mut settings = match Settings::load() {
        Err(err) => {
            eprintln!("{}", err);
            Settings::default()
        }
        Ok(s) => s,
    };

    let matches = get_command_line_matches();
    let (sub_name, sub_matches) = matches.subcommand();
    if sub_name == "set-storage-path" {
        let path_str = sub_matches.unwrap().value_of("path").unwrap();
        return set_storage_path(Path::new(path_str), &mut settings);
    }

    if settings.storage_path.components().next() == None {
        bail!("You must set the storage path.")
    }

    if !settings.storage_path.is_absolute() {
        bail!(
            "The configured storage path isn't absolute ({})",
            settings.storage_path.display()
        );
    }

    let db = Database::new(&settings.storage_path)?;

    settings.dry_run = sub_matches.unwrap().is_present("dry-run");

    match sub_name {
        "link" => Game::link_all(&db, &settings)?,
        "restore" => Game::restore_all(&db, &settings)?,
        "unlink" => Game::unlink_all(&db, &settings)?,
        "search" => {
            let keyword = sub_matches.unwrap().value_of("keyword").unwrap();
            db.search(&keyword);
        }
        "ignore" => {
            let id = sub_matches.unwrap().value_of("id").unwrap();
            if id.is_empty() {
                bail!("The game id must not be empty");
            }

            match db.games.iter().find(|g| g.id == id) {
                Some(g) => settings.ignore_game(&g)?,
                None => eprintln!("Couldn't find a game with id {}", id),
            }
        }
        "heed" => {
            let id = sub_matches.unwrap().value_of("id").unwrap();
            if id.is_empty() {
                bail!("The game id must not be empty");
            }

            match db.games.iter().find(|g| g.id == id) {
                Some(g) => settings.heed_game(&g)?,
                None => eprintln!("Couldn't find a game with id {}", id),
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
