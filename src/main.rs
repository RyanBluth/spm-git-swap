use repo::PackageRepo;
use simple_logger::SimpleLogger;
use structopt::StructOpt;

mod repo;
mod resolved;

/// A utility to clone repositories from .resolved files and update Git config.
#[derive(StructOpt, Debug)]
#[structopt(name = "spm-git-swap")]
struct Opt {
    /// The path to scan for .resolved files.
    #[structopt(parse(from_os_str))]
    path: std::path::PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    if let Err(e) = run(opt) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(opt: Opt) -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().init().unwrap();

    let mut package_repo = PackageRepo::new()?;
    package_repo.install(&opt.path)?;

    Ok(())
}
