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
use linker::Linker;
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
                .about("Set where game saves and meta data should be stored.")
                .arg(Arg::with_name("path").index(1).required(true)),
        )
        .subcommand(SubCommand::with_name("link").about(
            "Move game saves from their original locations to the \
             storage path and create links to their new location.",
        ))
        .subcommand(SubCommand::with_name("restore").about(
            "Creates links to game saves which have been moved to the \
             storage path.",
        ))
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

fn link(db: &Database, settings: &Settings) -> Result<()> {
    let movable = Game::all_with_saves(&db.games);
    println!(
        "Found {} games with saves in their standard locations",
        movable.len()
    );

    for game in movable {
        if let Err(e) = game.move_and_link(&settings.storage_path) {
            eprintln!("{}", e);
        }
    }

    Ok(())
}

fn restore(db: &Database, settings: &Settings) -> Result<()> {
    let restorable = Game::all_with_moved_saves(&db.games, &settings.storage_path);
    println!(
        "Found {} games with saves moved to {}",
        restorable.len(),
        settings.storage_path.display()
    );

    for game in restorable {
        if let Err(e) = game.restore(&settings.storage_path) {
            eprintln!("{}", e);
        }
    }

    Ok(())
}

fn run() -> Result<()> {
    let mut settings = Settings::load();

    let matches = get_command_line_matches();
    let (sub_name, sub_matches) = matches.subcommand();
    if sub_name == "set-storage-path" {
        let path_str = sub_matches.unwrap().value_of("path").unwrap();
        return set_storage_path(Path::new(path_str), &mut settings);
    }

    if !settings.storage_path.is_absolute() {
        bail!(
            "The configured storage path isn't absolute ({})",
            settings.storage_path.display()
        );
    }

    if Linker::check_reparse_privilege().is_err() {
        bail!(
            "You don't have the required privileges to create links. Try running as administrator"
        );
    }

    let db = Database::new(&settings.storage_path)?;

    match sub_name {
        "link" => link(&db, &settings)?,
        "restore" => restore(&db, &settings)?,
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
