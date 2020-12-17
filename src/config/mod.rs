use dotenv::dotenv;
// Adding the context method to errors:
use eyre::WrapErr;
use color_eyre::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
  pub db_path: String
}

impl Config {

  pub fn from_env() -> Result<Config> {
    dotenv().ok();
    let mut c = config::Config::new();
    c.merge(config::Environment::default())?;
    // The error has to be given a context for 
    // color_eyre to work here:
    c.try_into()
      .context("Loading configuration from env")
  }

}