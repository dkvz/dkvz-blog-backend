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
    pub article_id_or_url: String,
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

    // TODO: Could keep a cache of some article_url -> id

    // We may get several identical "visits" from the referrer
    // lines. We should only keep one.
    // TODO: The watcher mode should keep the last referrer
    // visits in some cache

    Ok(())
}

fn parse_log_line(line: &str, url_parser: &UrlParser) -> Result<Option<ParsedLogLine>> {
    // Thought I could split the lines but they're too weird
    // We have to use a regex. Also this is hyper specific to
    // Nginx, probably.
    lazy_static! {
        // IP, date, URL, status, referrer, user agent
        static ref RE_LOG_LINE: Regex =
            Regex::new(r#"^(\S+?)\s-.+\[(.+?)\]\s"\S{0,5}\s(\S+?)\s.+?"\s(\d+)\s.+?"(\S+?)"\s"(.+)"$"#).unwrap();
    }

    let captures = RE_LOG_LINE.captures(line);
    if captures.is_none() {
        return Err(eyre!("Log line couldn't be parsed"));
    }
    let caps = captures.unwrap();
    // Not sure this can happen but that's my excuse
    // for unleashing an unwrap party:
    if caps.len() < 7 {
        return Err(eyre!("Log line is missing values"));
    }

    // We use the HTTP status to determine if the article
    // "exists" though the site will currently send valid
    // responses with a 404 if the URL is not canonical,
    // sometimes. We'll just ignore that for now.
    let status: u32 = caps[4].parse().unwrap_or(0);
    if status >= 300 || status < 200 {
        return Ok(None);
    }

    // Check if we got an article or short URL in the
    // direct URL or referrer
    let parsed = url_parser
        .parse_url(&caps[3])
        .or_else(|| url_parser.parse_url(&caps[5]));

    // Doesn't match a direct article visit
    if parsed.is_none() {
        return Ok(None);
    }
    let parsed = parsed.unwrap();

    // We have a visit, gather the data to save it
    let date = time_utils::parse_nginx_log_date(&caps[2]);
    if date.is_none() {
        return Err(eyre!(
            "The date could not be parsed - Not supposed to happen"
        ));
    }

    Ok(Some(ParsedLogLine {
        article_id_or_url: parsed.1,
        article_type: parsed.0,
        timestamp: date.unwrap().timestamp(),
        client_ip: String::from(&caps[1]),
        client_ua: String::from(&caps[6]),
    }))
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

    #[test]
    fn can_parse_url_log_line() {
        let line = r###"2001:4860:4860::8844 - - [17/Jan/2026:09:23:18 +0100] "GET /articles/config_zsh_minimale_avec_starship HTTP/2.0" 206 1580 "-" "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/612.17 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/612.17""###;
        let url_parser = UrlParser::from("articles", "breves").unwrap();
        let parsed = parse_log_line(line, &url_parser).unwrap();
        let data = parsed.unwrap();

        assert_eq!("2001:4860:4860::8844", data.client_ip);
        assert_eq!(1768638198, data.timestamp);
        assert_eq!("config_zsh_minimale_avec_starship", data.article_id_or_url);
        assert_eq!(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/612.17 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/612.17",
            data.client_ua
        );
        assert_eq!(ArticleType::Article, data.article_type);
    }

    #[test]
    fn can_parse_referrer_log_line() {
        let line = r###"8.8.4.4 - - [20/Jan/2026:09:30:01 +0100] "POST /assets/shorts/image.png HTTP/1.1" 200 12907 "https://dkvz.eu/breves/176" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36""###;
        let url_parser = UrlParser::from("articles", "breves").unwrap();
        let parsed = parse_log_line(line, &url_parser).unwrap();
        let data = parsed.unwrap();

        assert_eq!("8.8.4.4", data.client_ip);
        assert_eq!(1768897801, data.timestamp);
        assert_eq!("176", data.article_id_or_url);
        assert_eq!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            data.client_ua
        );
        assert_eq!(ArticleType::Short, data.article_type);
    }
}
