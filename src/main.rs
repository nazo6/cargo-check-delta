use std::{
    collections::{HashMap, HashSet},
    time::SystemTime,
};

use dashmap::DashMap;
use ignore::{WalkBuilder, types::TypesBuilder};

fn main() {
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();
    let target_dir = &metadata.target_directory;

    let db_path = target_dir.join("cargo-check-delta.json");
    let old_db: HashMap<String, SystemTime> = match std::fs::read(&db_path) {
        Ok(str) => serde_json::from_slice(&str).unwrap_or_default(),
        Err(_e) => HashMap::new(),
    };

    let mut rust_types = TypesBuilder::new();
    rust_types.add("rust", "*.rs").unwrap();
    rust_types.select("rust");
    let rust_types = rust_types.build().unwrap();

    let mut walk = WalkBuilder::new("./");
    walk.types(rust_types);
    let walk = walk.build_parallel();

    let new_db = DashMap::new();

    walk.run(|| {
        Box::new(|entry| {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        if let Ok(modified) = metadata.modified() {
                            new_db.insert(entry.path().to_string_lossy().to_string(), modified);
                        }
                    }
                }
            }
            ignore::WalkState::Continue
        })
    });

    let mut changed_crates = vec![];

    let (added, removed, modified) = diff_dbs(&old_db, &new_db);
    let packages = metadata.workspace_packages();
    for changed_path in added.iter().chain(removed.iter()).chain(modified.iter()) {
        let changed_path = std::fs::canonicalize(changed_path).unwrap();
        for package in &packages {
            let crate_path = package.manifest_path.parent().unwrap();
            if changed_path.starts_with(crate_path) {
                changed_crates.push(crate_path);
            }
        }
    }

    eprintln!("added: {:?}", added);
    eprintln!("removed: {:?}", removed);
    eprintln!("modified: {:?}", modified);
    eprintln!("crates: {:?}", changed_crates);

    let passed_args = std::env::args().collect::<Vec<_>>();
    // invoke cargo check with args
    for changed_crate in changed_crates {
        let mut cmd = std::process::Command::new("cargo");
        cmd.current_dir(changed_crate);
        let mut args = vec!["check".to_string()];
        if passed_args.len() >= 2 && passed_args[1] == "check-delta" {
            if passed_args.len() >= 3 {
                args.extend_from_slice(&passed_args[2..]);
            }
        } else if !passed_args.is_empty() {
            args.extend_from_slice(&passed_args[1..]);
        }
        cmd.args(&args);
        eprintln!("cargo {} [at {}]", args.join(" "), changed_crate);
        let status = cmd.status().unwrap();
        if !status.success() {
            std::process::exit(status.code().unwrap());
        }
    }

    std::fs::write(&db_path, serde_json::to_vec(&new_db).unwrap()).unwrap();
}

fn diff_dbs(
    old_db: &HashMap<String, SystemTime>,
    new_db: &DashMap<String, SystemTime>,
) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
    let mut added = HashSet::new();
    let mut removed = HashSet::new();
    let mut modified = HashSet::new();

    for v in new_db.into_iter() {
        let (path, new_time) = v.pair();
        match old_db.get(path) {
            Some(old_time) => {
                if new_time > old_time {
                    modified.insert(path.clone());
                }
            }
            None => {
                added.insert(path.clone());
            }
        }
    }

    for path in old_db.keys() {
        if !new_db.contains_key(path) {
            removed.insert(path.clone());
        }
    }

    (added, removed, modified)
}
