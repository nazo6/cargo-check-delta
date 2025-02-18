use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Db {
    pub last_update: SystemTime,
    pub files: HashMap<String, SystemTime>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            last_update: SystemTime::now(),
            files: HashMap::new(),
        }
    }

    pub fn read_from_path(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let str = std::fs::read(path)?;
        Ok(serde_json::from_slice(&str)?)
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = serde_json::to_vec(self)?;
        Ok(std::fs::write(path, bytes)?)
    }

    pub fn diff(&self, other: &Self) -> DbDiff {
        let mut this_only = HashSet::new();
        let mut different_value = HashSet::new();
        let mut other_only = HashSet::new();

        for (key, value) in &self.files {
            if let Some(other_value) = other.files.get(key) {
                if other_value != value {
                    different_value.insert(key.clone());
                }
            } else {
                this_only.insert(key.clone());
            }
        }

        for key in other.files.keys() {
            if !self.files.contains_key(key) {
                other_only.insert(key.clone());
            }
        }

        DbDiff {
            this_only,
            different_value,
            other_only,
        }
    }
}

#[derive(Debug)]
pub struct DbDiff {
    pub this_only: HashSet<String>,
    pub different_value: HashSet<String>,
    pub other_only: HashSet<String>,
}
