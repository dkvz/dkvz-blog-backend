/**
* Process stats from log files
* and inserts them in the stats
* database
*/
mod config;
mod db;
mod utils;

use crate::config::Config;
use crate::db::Pool;
use crate::db::entities::*;
use color_eyre::Result;
use dotenv::dotenv;
use getopts::Options;
use r2d2_sqlite::SqliteConnectionManager;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

// Copy pasted this from getopts doc.
pub fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} LOG_FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optflag("w", "watch", "Watch the given log file for changes");
    opts.optflag("h", "help", "Program usage");
    let opt_matches = opts.parse(args)?;
    if opt_matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }

    // We also need the filename:
    let file_arg = if opt_matches.free.len() > 1 {
        opt_matches.free[1].clone()
    } else {
        print_usage(&program, opts);
        return Ok(());
    };

    println!("Processing file {}...", file_arg);

    let config = Config::from_env().expect("Configuration (environment or .env file) is missing");
    let manager = SqliteConnectionManager::file(&config.db_path);
    let pool = Pool::new(manager).expect("Database connection failed");

    // let file = File::open(file_arg)?;
    // let reader = BufReader::new(file);
    // for line in reader.lines() {
    //     println!("{}", line?);
    // }

    Ok(())
}
