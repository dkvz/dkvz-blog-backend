mod config;
use color_eyre::Result;
// I think we have to add crate here because
// of the other crate named "config" that we
// use as a dependency.
use crate::config::Config;

fn main() -> Result<()> {
    let config = Config::from_env()
        .expect("Configuration (environment or .env file) is missing");

    println!("Found config: {:?}", config);

    Ok(())
}
