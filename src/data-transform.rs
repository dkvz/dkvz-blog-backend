#![allow(dead_code)]
mod config;
mod db;
mod utils;

use std::env;
use color_eyre::Result;
use dotenv::dotenv;
use log::{debug, error, info};
use r2d2_sqlite::{self, SqliteConnectionManager};
use crate::db::Pool;
use crate::db::Order;
use crate::config::Config;

/**
 * Binary meant to perform hardcoded database migrations
 */
fn main() -> Result<()> {
  dotenv().ok();
  env_logger::init();

  let config = Config::from_env()
    .expect("Configuration (environment or .env file) is missing");
  debug!("Current config: {:?}", config);

  let manager = SqliteConnectionManager::file(&config.db_path);
  let pool = Pool::new(manager)
    .expect("Database connection failed");

  // Load IDs of every article, including non-published ones:
  let article_ids = db::all_articles_and_shorts_ids(&pool, Order::Asc, false)?;
  println!("{:?}", article_ids);

  // Do we need to rebuild the fulltext index? I'd say no.
  // But important to keep in mind for other transforms.

  Ok(())
}