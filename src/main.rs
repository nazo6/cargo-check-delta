use std::sync::LazyLock;
use std::{collections::HashSet, io::Write};

use cargo_metadata::Metadata;
use clap::{Parser, ValueEnum};
use db::Db;
use ignore::{WalkBuilder, types::TypesBuilder};

mod db;

#[derive(Debug, Clone, ValueEnum, Copy)]
enum LogType {
    StdErr,
    File,
    None,
}

#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum Cli {
    CheckDelta(CheckDelta),
}
#[derive(Debug, clap::Args)]
#[command(version, about, long_about = None)]
struct CheckDelta {
    /// cargo subcommand to invoke
    #[arg(short = 's', default_value_t = {"check".to_string()})]
    cargo_subcommand: String,

    #[arg(value_enum, short, default_value_t = LogType::StdErr)]
    log_type: LogType,

    /// Ignore old db
    #[arg(short, long, default_value_t = false)]
    reset: bool,

    /// Args passed to cargo subcommand
    #[arg(allow_hyphen_values = true)]
    args: Vec<String>,
}

static METADATA: LazyLock<Metadata> =
    LazyLock::new(|| cargo_metadata::MetadataCommand::new().exec().unwrap());

fn main() {
    let Cli::CheckDelta(cli) = Cli::parse();

    let target_dir = &METADATA.target_directory;

    let db_path = target_dir.join("cargo-check-delta.json");
    let mut old_db = if cli.reset {
        Db::new()
    } else {
        Db::read_from_path(&db_path).unwrap_or(Db::new())
    };

    let mut new_db = Db::new();

    if let Ok(since_last_update) = new_db.last_update.duration_since(old_db.last_update) {
        if since_last_update > std::time::Duration::from_secs(300) {
            log(cli.log_type, "db is too old, ignoring old db.");
            old_db = Db::new();
        }
    }

    let walk = {
        let mut rust_types = TypesBuilder::new();
        rust_types.add("rust", "*.rs").unwrap();
        rust_types.select("rust");
        let rust_types = rust_types.build().unwrap();

        let mut walk = WalkBuilder::new("./");
        walk.types(rust_types);
        walk.build()
    };
    for entry in walk.flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                if let Ok(modified) = metadata.modified() {
                    new_db
                        .files
                        .insert(entry.path().to_string_lossy().to_string(), modified);
                }
            }
        }
    }

    let mut changed_crates = HashSet::new();

    let diff = old_db.diff(&new_db);

    log(cli.log_type, &format!("changed_files: {:?}", diff));

    let packages = METADATA.workspace_packages();
    for changed_path in diff
        .this_only
        .iter()
        .chain(diff.different_value.iter())
        .chain(diff.other_only.iter())
    {
        let changed_path = std::fs::canonicalize(changed_path).unwrap();
        for package in &packages {
            let crate_path = package.manifest_path.parent().unwrap();
            if changed_path.starts_with(crate_path) {
                changed_crates.insert(crate_path);
            }
        }
    }

    log(
        cli.log_type,
        &format!("changed_crates: {:?}", changed_crates),
    );

    for changed_crate in changed_crates {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg(&cli.cargo_subcommand);
        cmd.args(&cli.args);
        cmd.current_dir(changed_crate);

        log(cli.log_type, &format!("running: {:?}", cmd));

        let status = cmd.status().unwrap();

        if !status.success() {
            std::process::exit(status.code().unwrap());
        }
    }

    new_db.save_to_path(db_path).unwrap();
}

fn log(log_type: LogType, message: &str) {
    match log_type {
        LogType::StdErr => eprintln!("{}", message),
        LogType::File => {
            let path = METADATA.target_directory.join("cargo-check-delta.log");
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap();
            writeln!(file, "{}", message).unwrap();
        }
        LogType::None => {}
    }
}
