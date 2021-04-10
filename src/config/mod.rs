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
  pub bind_address: String,
  pub message_queue_size: usize,
  // Rate limiter settings:
  pub rl_max_requests: u32,
  pub rl_max_requests_time: u32,
  pub rl_block_duration: u32,
  pub import_path: String
}

impl Config {

  pub fn from_env() -> Result<Config> {
    let mut c = config::Config::new();
    // RUST_LOG is already set in main.rs if it
    // was absent.
    // Let's set other default values. You have 
    // to use lowercase when compared to what's 
    // in the .env file.
    c.set_default("bind_address", "127.0.0.1:8080")?;
    // Used to set the queue size for sync_sender
    // (the Stats thread uses it):
    c.set_default("message_queue_size", 30)?;
    // Settings for the basic rate limiter I'm 
    // using:
    c.set_default("rl_max_requests", 120)?;
    c.set_default("rl_max_requests_time", 60)?;
    c.set_default("rl_block_duration", 60)?;
    // Default import path:
    c.set_default("import_path", "./import/")?;

    c.merge(config::Environment::default())?;
    // The error has to be given a context for 
    // color_eyre to work here:
    c.try_into()
      .context("Loading configuration from env")
  }

}