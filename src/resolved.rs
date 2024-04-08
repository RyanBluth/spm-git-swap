use glob::glob;
use log::info;

use std::{collections::{HashMap, HashSet}, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolvedError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Glob error: {0}")]
    Glob(#[from] glob::GlobError),

    #[error("Pattern error: {0}")]
    Pattern(#[from] glob::PatternError),

    #[error("Version not found in {0}. Version 1 and 2 are supported.")]
    VersionNotFound(Box<Path>),
}

pub fn parse_all_recursive(path: &Path) -> Result<Vec<v2::Pin>, ResolvedError> {
    let mut pins: HashMap<String, v2::Pin> = HashMap::new();
    for entry in glob(&format!("{}/**/Package.resolved", path.to_str().unwrap()))? {
        let path = entry?;
        for pin in parse(&path)?.pins {
            pins.insert(pin.location.clone(), pin);
        }
    }

    Ok(pins.into_values().collect())
}

pub fn parse(path: &Path) -> Result<v2::Resolved, ResolvedError> {
    info!("Parsing resolved file: {:?}", path);

    let contents = std::fs::read_to_string(path)?;
    let version = contents
        .lines()
        .rev() // Version seems to be at the bottom
        .find_map(|line| {
            if line.contains(r#""version""#) {
                let stripped = line
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == ':')
                    .collect::<String>();
                if stripped == "version:1" {
                    return Some(1);
                } else if stripped == "version:2" {
                    return Some(2);
                }
            }

            return None;
        });

    match version {
        Some(1) => {
            info!("Parsing as version 1");
            Ok(v1::parse(path)?.into())
        }
        Some(2) => {
            info!("Parsing as version 2");
            Ok(v2::parse(path)?)
        }
        _ => Err(ResolvedError::VersionNotFound(path.into())),
    }
}

pub mod v2 {
    use super::ResolvedError;
    use serde::{Deserialize, Serialize};
    use std::path::Path;

    #[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub enum Kind {
        RemoteSourceControl,
        LocalSourceControl,
        BinaryTarget,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Resolved {
        pub pins: Vec<Pin>,
        pub version: u8,
    }

    #[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
    pub struct Pin {
        pub identity: String,
        pub kind: Kind,
        pub location: String,
        pub state: State,
    }

    #[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
    pub struct State {
        pub revision: String,
        pub version: Option<String>,
    }

    pub(super) fn parse(path: &Path) -> Result<Resolved, ResolvedError> {
        let file = std::fs::read_to_string(path)?;
        let root: Resolved = serde_json::from_str(&file)?;
        Ok(root)
    }
}

mod v1 {
    use super::ResolvedError;
    use serde::{Deserialize, Serialize};
    use std::path::Path;

    #[derive(Debug, Serialize, Deserialize)]
    pub(super) struct Resolved {
        pub object: Object,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub(super) struct Object {
        pub pins: Vec<Pin>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub(super) struct Pin {
        pub package: String,
        pub repositoryURL: String,
        pub state: State,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub(super) struct State {
        pub branch: Option<String>,
        pub revision: String,
        pub version: Option<String>,
    }

    pub(super) fn parse(path: &Path) -> Result<Resolved, ResolvedError> {
        let file = std::fs::read_to_string(path)?;
        let root: Resolved = serde_json::from_str(&file)?;
        Ok(root)
    }
}

impl Into<v2::Resolved> for v1::Resolved {
    fn into(self) -> v2::Resolved {
        let pins = self
            .object
            .pins
            .into_iter()
            .map(|pin| {
                let identity = pin.package;
                let kind = v2::Kind::RemoteSourceControl;
                let location = pin.repositoryURL;
                let state = v2::State {
                    revision: pin.state.revision,
                    version: pin.state.version,
                };
                v2::Pin {
                    identity,
                    kind,
                    location,
                    state,
                }
            })
            .collect();
        v2::Resolved { pins, version: 2 }
    }
}
