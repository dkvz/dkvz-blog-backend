mod config;
mod db;
use db::{ Pool, all_tags, comment_count };
use color_eyre::Result;
use r2d2_sqlite::{self, SqliteConnectionManager};
// I think we have to add crate here because
// of the other crate named "config" that we
// use as a dependency.
use crate::config::Config;

fn main() -> Result<()> {
    let config = Config::from_env()
        .expect("Configuration (environment or .env file) is missing");

    let manager = SqliteConnectionManager::file(&config.db_path);
    let pool = Pool::new(manager)
        .expect("Database connection failed");

    let tags = all_tags(&pool)?;
    let count = comment_count(&pool, 110)?;

    println!("Found config: {:?}", config);
    println!("Found tags: {}", tags.len());
    println!("Comment count for article 110: {}", count);

    Ok(())
}
