mod config;

use crate::config::Config;
use dotenv::dotenv;
use eyre::{Result, eyre};
/**
* These are integration tests that require a working config
* and ip2location Lite databases for ipv4 and ipv6.
* As such, they're disabled by default.
*
* TODO: Remember to set test = false in Cargo.toml
* I set it to true to get my LSP to work.
*/

fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = Config::from_env().expect("Configuration (environment or .env file) is missing");

    Err(eyre!("I failed"))
}
