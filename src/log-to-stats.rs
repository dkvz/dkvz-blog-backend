/**
* Process stats from log files and inserts them in the
* stats database. Probably has absolutely no use to
* anyone but me.
*/
mod config;
mod db;
mod utils;

use crate::config::Config;
use crate::db::Pool;
use crate::db::entities::*;
use color_eyre::Result;
use dotenv::dotenv;
use eyre::eyre;
use getopts::Options;
use lazy_static::lazy_static;
use r2d2_sqlite::SqliteConnectionManager;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug)]
enum ArticleType {
    Short,
    Article,
}

#[derive(Debug)]
struct ParsedLogLine {
    pub id_or_url: String,
    pub article_type: ArticleType,
    pub client_ip: String,
    pub client_ua: String,
    pub timestamp: i64,
}

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

fn parse_log_line(line: &str, pool: &Pool) -> Result<Option<ParsedLogLine>> {
    // Thought I could split the lines but they're too weird
    // We have to use a regex. Also this is hyper specific to
    // Nginx, probably.
    lazy_static! {
        // IP, date, URL, referrer, user agent
        static ref RE_LOG_LINE: Regex =
            Regex::new(r#"^(\S+?)\s-.+\[(.+?)\]\s\"\S{0,5}\s(\S+?)\s.+?\".+?\"(\S+?)\"\s\"(.+)\"$"#).unwrap();
    }

    let captures = RE_LOG_LINE.captures(line);
    if captures.is_none() {
        return Err(eyre!("Log line couldn't be parsed"));
    }
    let caps = captures.unwrap();
    // Not sure this can happen but that's my excuse
    // for unleashing an unwrap party:
    if caps.len() < 6 {
        return Err(eyre!("Log line is missing values"));
    }

    let date = &caps[2];

    Err(eyre!("Not implemented"))
}
