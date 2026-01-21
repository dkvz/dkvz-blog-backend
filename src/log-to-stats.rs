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
use crate::utils::time_utils;
use color_eyre::Result;
use dotenv::dotenv;
use eyre::Context;
use eyre::eyre;
use getopts::Options;
use lazy_static::lazy_static;
use r2d2_sqlite::SqliteConnectionManager;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, PartialEq)]
enum ArticleType {
    Short,
    Article,
}

#[derive(Debug)]
struct ParsedLogLine {
    pub article_id: i32,
    pub article_type: ArticleType,
    pub client_ip: String,
    pub client_ua: String,
    pub timestamp: i64,
}

// Needed because I wanted the shorts and
// articles URLs to by determined based on
// the config file - Guess I was bored.
struct UrlParser {
    url_regex: Regex,
    articles_root: String,
    shorts_root: String,
}

impl UrlParser {
    pub fn from(articles_root: &str, shorts_root: &str) -> Result<Self> {
        let re = Regex::new(&format!(r#"/({}|{})/(.+?)/?$"#, articles_root, shorts_root))?;

        Ok(Self {
            url_regex: re,
            articles_root: String::from(articles_root),
            shorts_root: String::from(shorts_root),
        })
    }

    pub fn parse_url(&self, url: &str) -> Option<(ArticleType, String)> {
        self.url_regex.captures(url).and_then(|caps| {
            if caps[1] == self.shorts_root {
                return Some((ArticleType::Short, String::from(&caps[2])));
            } else if caps[1] == self.articles_root {
                return Some((ArticleType::Article, String::from(&caps[2])));
            }
            None
        })
    }
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

    let url_parser = UrlParser::from(&config.site_articles_root, &config.site_shorts_root)
        .context("UrlParser creation")?;

    // let file = File::open(file_arg)?;
    // let reader = BufReader::new(file);
    // for line in reader.lines() {
    //     println!("{}", line?);
    // }

    Ok(())
}

fn parse_log_line(
    line: &str,
    pool: &Pool,
    url_parser: &UrlParser,
) -> Result<Option<ParsedLogLine>> {
    // Thought I could split the lines but they're too weird
    // We have to use a regex. Also this is hyper specific to
    // Nginx, probably.
    lazy_static! {
        // IP, date, URL, referrer, user agent
        static ref RE_LOG_LINE: Regex =
            Regex::new(r#"^(\S+?)\s-.+\[(.+?)\]\s\"\S{0,5}\s(\S+?)\s.+?\".+?\"(\S+?)\"\s\"(.+)\"$"#).unwrap();
        static ref RE_URL: Regex =
            Regex::new(r#"/(breves|articles)"#).unwrap();
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

    // Check if we got an article or short URL in the
    // direct URL or referrer
    let parsed = url_parser
        .parse_url(&caps[3])
        .or_else(|| url_parser.parse_url(&caps[4]));

    // Doesn't match a direct article visit
    if parsed.is_none() {
        return Ok(None);
    }

    // Check if the id or url does exist - We could cache
    // some of these or devise a way to find out which visits
    // are correct. Using 404 errors doesn't really work for
    // now as they can actually return true content (lol).
    // However I'll accept these false positives and just look
    // for 2xx responses and just log their stats without
    // checking the database.

    // We have a visit, gather the data to save it
    let date = time_utils::parse_nginx_log_date(&caps[2]);

    Err(eyre!("Not implemented"))
}

#[cfg(test)]
mod log_to_stats_tests {
    use super::*;

    #[test]
    fn url_parser_matches_content_url() {
        let url1 = "/breves/127/";
        let url2 = "/breves/127";
        let url3 = "/articles/article_slug_here";

        let parser = UrlParser::from("articles", "breves").unwrap();
        let parsed1 = parser.parse_url(url1).unwrap();
        let parsed2 = parser.parse_url(url2).unwrap();
        let parsed3 = parser.parse_url(url3).unwrap();

        assert_eq!(ArticleType::Short, parsed1.0);
        assert_eq!(ArticleType::Short, parsed2.0);
        assert_eq!(ArticleType::Article, parsed3.0);
        assert_eq!("127", parsed1.1);
        assert_eq!("127", parsed2.1);
        assert_eq!("article_slug_here", parsed3.1);
    }
}
