// #[macro_use]
use super::manifest::Manifest;
use std::fs;
use std::path::PathBuf;
use tracing::{event, Level};

/// Holds the connection information that redis should use for connecting.
// #[derive(Clone, Debug)]
#[derive(Clone, Debug)]
pub struct Context {
    pub working_dir: PathBuf,      // Where to execute.
    pub config_file_path: PathBuf, // the config file path
    pub log_file_path: PathBuf,    // the log file path, empty if not logging to a file
}

impl Context {
    pub fn new(config_file_path: PathBuf, working_dir: PathBuf, log_file_path: PathBuf) -> Context {
        Context {
            working_dir,
            config_file_path,
            log_file_path,
        }
    }
}

pub fn init_cli(manifest_name: &str) -> std::result::Result<(Manifest, tracing_appender::non_blocking::WorkerGuard, Context), anyhow::Error> {

    let manifest;
    let guard;
    let path = PathBuf::from(manifest_name);
    if path.exists() {
        println!("Using config file: {:?}", fs::canonicalize(&path)?);
        (manifest, guard) = Manifest::load(manifest_name)?;
    } else {
        println!(
            "default manifest missing: {}, using default values",
            manifest_name
        );
        (manifest, guard) = Manifest::from_default(&manifest_name)?;
        manifest.save()?;
    }

    event!(Level::INFO, "starting");
    event!(Level::DEBUG, "parameters: {:?}", manifest);

    let ctx = Context::new(
        PathBuf::from(manifest_name),
        PathBuf::from("./"),
        manifest.logging.filename.to_owned(),
    );


    Ok((manifest, guard, ctx))
}
