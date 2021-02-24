// Adding the context method to errors:
use eyre::WrapErr;
use color_eyre::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
  pub db_path: String,
  pub stats_db_path: String,
  pub iploc_path: String,
  pub wordlist_path: String,
  pub bind_address: String
}

impl Config {

  pub fn from_env() -> Result<Config> {
    let mut c = config::Config::new();
    c.merge(config::Environment::default())?;
    // The error has to be given a context for 
    // color_eyre to work here:
    c.try_into()
      .context("Loading configuration from env")
  }

}