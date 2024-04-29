use std::{path, process::Command};

use auth_git2::GitAuthenticator;
use git2::Config;
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
    git: GitAuthenticator,
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
            git: GitAuthenticator::default()
                .try_cred_helper(true)
                .add_default_username()
                .try_ssh_agent(true)
                .add_default_ssh_keys(),
        })
    }

    pub fn wipe(&self) -> Result<(), PackageRepoError> {
        info!(
            "Wiping checkouts directory: {}",
            self.checkouts_dir().display()
        );
        std::fs::remove_dir_all(self.checkouts_dir())?;
        Ok(())
    }

    pub fn install(&mut self, path: &path::Path) -> Result<(), PackageRepoError> {
        info!("Scanning directory: {:?} for Package.resovled", path);
        let pins = parse_all_recursive(path)?;

        for pin in pins {
            info!("Cloning: {:?}", pin.identity);
            if let Err(error) = self.clone(&pin) {
                log::error!(
                    "Error cloning {} at: {}. {}",
                    pin.identity,
                    pin.location,
                    error,
                );
            }
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

        let mut repo_url = pin.location.clone();

        if pin.location.starts_with("https://github.com/") {
            let parts: Vec<&str> = pin.location.split('/').collect();
            let repo_name = parts[parts.len() - 1];
            let user_name = parts[parts.len() - 2];
            repo_url = format!("git@github.com:{}/{}", user_name, repo_name);
            info!(
                "Converting https to ssh for {}. Converted to {}",
                pin.location, repo_url
            );
        }

        let version = pin
            .state
            .version
            .clone()
            .unwrap_or_else(|| String::from("NO_VERSION"));

        let path = self.checkouts_dir().join(pin.identity.clone());
        let git_path = path.join(".git");

      

        Self::remove_global_git_proxy(&path.display().to_string())?;

        if path.exists() && git_path.exists() {
            info!("{} already exists, fetching", pin.identity);

            let repo = git2::Repository::open(&path)?;
            let mut remote = repo.find_remote("origin")?;

            self.git
                .fetch(&repo, &mut remote, &["refs/heads/*:refs/heads/*"], None)?;

            Self::set_global_git_proxy(&pin.location, &path.display().to_string())?;

            return Ok(());
        } else {
            info!("Cloning {} at {}", pin.identity, pin.location);
        }

        self.git.clone_repo(&repo_url, &path).or_else(|err| {
            if path.exists() {
                info!("Removing {} due to error cloning", path.display());
                if let Err(deleter_error) = std::fs::remove_dir_all(&path) {
                    log::error!(
                        "Error deleting {} after error cloning: {}. You may need to manually delete this directory.",
                        path.display(),
                        deleter_error
                    );
                }
            }
            Err(err)
        })?;

        info!(
            "Cloned {} , version {} at revision: {}",
            pin.identity, version, pin.state.revision
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

    fn set_global_git_proxy(repo_url: &str, proxy_path: &str) -> Result<(), PackageRepoError> {

        let config_value = format!("url.{}.insteadOf", proxy_path);
        
        let mut config =  Config::open_default()?;

        config.set_str(&config_value, repo_url)?;

        Ok(())
    }

    fn remove_global_git_proxy(proxy_path: &str) -> Result<(), PackageRepoError> {
       
        let config_value = format!("url.{}.insteadOf", proxy_path);
        
        let mut config =  Config::open_default()?;

        if config.get_entry(&config_value).is_ok() {
            config.remove(&config_value)?;
        }

        Ok(())
    }
}
