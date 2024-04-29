use repo::PackageRepo;
use simple_logger::SimpleLogger;
use structopt::StructOpt;

mod repo;
mod resolved;

/// A utility to clone repositories from .resolved files and update Git config.
#[derive(StructOpt, Debug)]
#[structopt(name = "spm-git-swap")]
enum Opt {

    /// Install packages from .resolved files.
    Install {
         /// The path to scan for .resolved files.
        #[structopt(parse(from_os_str))]
        path: std::path::PathBuf,
    },

    /// Wipe cached repositories.
    Wipe
   
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
    
    match opt {
        Opt::Install { path } => {
            package_repo.install(&path)?;
        
        },
        Opt::Wipe => {
            package_repo.wipe()?;
        },
    }

    Ok(())
}
