// Adding the context method to errors:
use eyre::WrapErr;
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::convert::From;

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
  pub import_path: String,
  pub template_dir: String,
  // Used to generate the RSS fields
  // and server-side-render articles:
  pub site_title: String,
  pub site_root: String,
  pub site_rss_full_url: String,
  pub site_articles_root: String,
  pub site_shorts_root: String,
  pub site_description: String
}

// Looks redundant but I thought having another 
// struct would be better than moving all of this
// info around the app_state, especially since 
// there could be sensible info in the config.
#[derive(Serialize)]
pub struct SiteInfo {
  pub title: String,
  pub root: String,
  pub rss_full_url: String,
  pub articles_root: String,
  pub shorts_root: String,
  pub description: String
}

// I'm using From so that transforming into 
// SiteInfo supposedely drops all of the other
// into since a "move" is obligatory here.
impl From<Config> for SiteInfo {
  fn from(config: Config) -> Self {
    Self {
      title: config.site_title,
      root: config.site_root,
      rss_full_url: config.site_rss_full_url,
      articles_root: config.site_articles_root,
      shorts_root: config.site_shorts_root,
      description: config.site_description
    }
  }
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
    // Default template directory:
    c.set_default("template_dir", "./templates")?;
    // Default website URLs and OpenGraph etc.
    // config:
    c.set_default("site_title", "Blog des gens compliqués")?;
    // Should never have a trailing slash or THINGS WILL BREAK.
    c.set_default("site_root", "https://dkvz.eu")?;
    c.set_default("site_rss_full_url", "https://dkvz.eu/rss.xml")?;
    c.set_default("site_articles_root", "articles")?;
    c.set_default("site_shorts_root", "breves")?;
    c.set_default("site_description", "Blog bizarre d'un humble consultant en progress bars.")?;

    c.merge(config::Environment::default())?;
    // The error has to be given a context for 
    // color_eyre to work here:
    c.try_into()
      .context("Loading configuration from env")
  }

}