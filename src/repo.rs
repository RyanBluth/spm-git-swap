use std::{path, process::Command};

use git2::{self, build::RepoBuilder, FetchOptions};
use log::{info, warn};

use thiserror::Error;

use crate::resolved::{parse_all_recursive, v2};

#[derive(Error, Debug)]
pub enum PackageRepoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Resolve error: {0}")]
    Resolve(#[from] crate::resolved::ResolvedError),

    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Git config error: {0}")]
    GitConfig(String),
}

const CHECKOUTS_DIR: &str = "checkouts";

pub struct PackageRepo {
    dir: path::PathBuf,
}

impl PackageRepo {
    pub fn new() -> Result<Self, PackageRepoError> {
        let working_dir = std::env::current_dir()?;
        let repo_dir = std::env::var("REPO_DIR").unwrap_or_else(|_| {
            warn!("REPO_DIR not set, using current directory({}/swifter-package-manager/checkouts) to store packages", working_dir.display());
            working_dir.join(path::Path::new("swifter-package-manager")).display().to_string()
        });
        let repo_dir = path::Path::new(&repo_dir);

        if !repo_dir.exists() {
            info!("Creating repo directory at {}", repo_dir.display());
            std::fs::create_dir_all(repo_dir)?;
        }

        let checkouts_dir = repo_dir.join(path::Path::new(CHECKOUTS_DIR));
        if !checkouts_dir.exists() {
            info!(
                "Creating checkouts directory at {}",
                checkouts_dir.display()
            );
            std::fs::create_dir_all(checkouts_dir)?;
        }

        Ok(Self {
            dir: repo_dir.to_path_buf(),
        })
    }

    pub fn install(&mut self, path: &path::Path) -> Result<(), PackageRepoError> {
        info!("Scanning directory: {:?} for Package.resovled", path);
        let pins = parse_all_recursive(path)?;

        for pin in pins {
            info!(
                "Cloning: {:?} at revision {}",
                pin.identity, pin.state.revision
            );
            self.clone(&pin)?;
        }

        Ok(())
    }
}

impl PackageRepo {
    fn clone(&mut self, pin: &v2::Pin) -> Result<(), PackageRepoError> {

        if pin.kind != v2::Kind::RemoteSourceControl {
            info!("Skipping {} as it is not a git repo", pin.identity);
            return Ok(());
        }

        let version = pin
            .state
            .version
            .clone()
            .unwrap_or_else(|| String::from("NO_VERSION"));

        let path = self.checkouts_dir().join(pin.identity.clone());

        if path.exists() {
            info!(
                "Revsion {} for {} already exists, fetching",
                pin.state.revision, pin.identity
            );

            let repo = git2::Repository::open(&path)?;

            let mut fetch_options = FetchOptions::new();
            fetch_options.download_tags(git2::AutotagOption::All);

            let mut remote = repo.find_remote("origin")?;
            remote.fetch(
                &["refs/heads/*:refs/heads/*"],
                Some(&mut fetch_options),
                None,
            )?;

            return Ok(());
        }

        RepoBuilder::new().clone(&pin.location, &path)?;

        info!(
            "Cloned {} at revision {}, version {}",
            pin.identity, pin.state.revision, version
        );

        info!(
            "Setting global git proxy for {} to {}",
            pin.location,
            &path.display()
        );

        Self::set_global_git_proxy(&pin.location, &path.display().to_string())?;

        Ok(())
    }

    fn checkouts_dir(&self) -> path::PathBuf {
        self.dir.join(path::Path::new(CHECKOUTS_DIR))
    }

    fn set_global_git_proxy(
        repo_url: &str,
        proxy_script_path: &str,
    ) -> Result<(), PackageRepoError> {
        let config_value = format!("url.{}.insteadOf", proxy_script_path);

        let output = Command::new("git")
            .args(&["config", "--global", "--add", &config_value, repo_url])
            .output()?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(PackageRepoError::GitConfig(error_message.to_string()));
        }

        Ok(())
    }
}
