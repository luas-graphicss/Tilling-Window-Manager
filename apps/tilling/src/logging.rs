use std::error::Error;

use tracing::Level;

pub fn start(verbose: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
    let level = if verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(level)
        .try_init()
}
