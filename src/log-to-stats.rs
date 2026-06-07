/**
* Process stats from log files and inserts them in the
* stats database. Probably has absolutely no use to
* anyone but me.
*
* ALWAYS BACKUP THE STATS DB BEFORE RUNNING
*/
use color_eyre::Result;
use dkvz_blog_backend::config::Config;
use dkvz_blog_backend::db::entities::*;
use dkvz_blog_backend::db::{Pool, article_by_url, insert_article_stat, last_article_stats};
use dkvz_blog_backend::stats::ip_location::GeoInfo;
use dkvz_blog_backend::stats::ip_location::IpLocator;
use dkvz_blog_backend::stats::pseudonymize;
use dkvz_blog_backend::stats::pseudonymizer::WordlistPseudoyimizer;
use dkvz_blog_backend::utils::ip_utils;
use dkvz_blog_backend::utils::text_utils;
use dkvz_blog_backend::utils::time_utils;
use dotenv::dotenv;
use eyre::Context;
use eyre::eyre;
use getopts::Options;
use lazy_static::lazy_static;
use log::{debug, error};
use r2d2_sqlite::SqliteConnectionManager;
use regex::Regex;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::From;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::IpAddr;

const LAST_STATS_COUNT: usize = 30;
// Duration window meant to identify duplicate stat entries:
const EXPIRED_STAT_SECONDS: i64 = 60;
const URL_MAX_LENGTH: usize = 120;

#[derive(Debug, PartialEq)]
enum ArticleType {
    Short,
    Article,
}

#[derive(Debug)]
struct ParsedLogLine {
    pub article_id_or_url: String,
    // Not sure we actually need the ArticleType
    pub article_type: ArticleType,
    pub client_ip: String,
    pub client_ua: String,
    pub timestamp: i64,
}

impl From<ArticleStat> for StatHistoryItem {
    fn from(value: ArticleStat) -> Self {
        Self {
            article_id: value.article_id,
            timestamp: value.date.unwrap_or(0),
            client_ip: value.client_ip,
            client_ua: value.client_ua,
        }
    }
}

#[derive(Debug)]
pub struct StatHistoryItem {
    pub article_id: i32,
    pub timestamp: i64,
    pub client_ip: String,
    pub client_ua: String,
}

// There's a lot of stuff in this file, should be split
// up but then I need to refactor the whole project into
// a Cargo workspace first.

pub struct StatsHistory {
    entries: VecDeque<StatHistoryItem>,
    capacity: usize,
}

impl StatsHistory {
    pub fn from(initial: Vec<StatHistoryItem>) -> Self {
        let entries = VecDeque::from(initial);
        Self {
            capacity: entries.len(),
            entries,
        }
    }

    pub fn add(&mut self, entry: StatHistoryItem) {
        // Supposed to use push_back
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn is_duplicate(&self, s: &StatHistoryItem) -> Result<bool> {
        if !self.entries.is_empty() {
            // Oh crap, date is an Option? Oh well
            if s.timestamp < self.entries[self.entries.len() - 1].timestamp {
                // The entry is in the past compared to the earliest
                // entry in the history, we currently ignore these
                // with an error.
                return Err(eyre!(
                    "We currently don't allow inserting stats older than the latest entry"
                ));
            }

            for e in &self.entries {
                // Only consider entries with the same article id,
                // user agent and ip address
                if s.article_id == e.article_id
                    && s.client_ip == e.client_ip
                    && s.client_ua == e.client_ua
                {
                    // Candidate stat time is supposed to be higher
                    // than any history stat time.
                    // This is already sort of checked for above but
                    // since I order by ID and not date I might as
                    // well double check.
                    let diff = s.timestamp - e.timestamp;
                    if diff >= 0 && diff <= EXPIRED_STAT_SECONDS {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
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
        let re = Regex::new(&format!(
            r#"/({}|{})/([^/]+?)/?$"#,
            articles_root, shorts_root
        ))?;

        Ok(Self {
            url_regex: re,
            articles_root: String::from(articles_root),
            shorts_root: String::from(shorts_root),
        })
    }

    pub fn parse_url(&self, url: &str) -> Option<(ArticleType, String)> {
        self.url_regex.captures(url).and_then(|caps| {
            let article_type = if caps[1] == self.shorts_root {
                Some(ArticleType::Short)
            } else if caps[1] == self.articles_root {
                Some(ArticleType::Article)
            } else {
                None
            };
            article_type.map(|a| {
                // Truncate the URL part
                // My utility function takes a weird mutable string
                // as its argument so here we go
                let mut url = String::from(&caps[2]);
                text_utils::truncate_utf8(&mut url, URL_MAX_LENGTH);
                (a, url)
            })
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
    let manager = SqliteConnectionManager::file(&config.stats_db_path);
    let manager_main = SqliteConnectionManager::file(&config.db_path);
    let pool = Pool::new(manager).expect("Database connection failed");
    let pool_main = Pool::new(manager_main).expect("Database connection failed");

    // Create the ip locator:
    let mut iploc = IpLocator::open(&config.iploc_path)?;
    // Create the pseudonymizer:
    let mut pseudonymizer = WordlistPseudoyimizer::open(&config.wordlist_path)?;

    let url_parser = UrlParser::from(&config.site_articles_root, &config.site_shorts_root)
        .context("UrlParser creation")?;

    let last_stats = last_article_stats(&pool, LAST_STATS_COUNT)?;
    println!(
        "Populating last stats history with {} items",
        last_stats.len()
    );
    let mut stats_history: StatsHistory =
        StatsHistory::from(last_stats.into_iter().map(|s| s.into()).collect());

    // Only referrer based visits risk being duplicated but
    // I'm applying it to every entry because that's easier

    let file = File::open(file_arg)?;
    let reader = BufReader::new(file);
    let mut last_article: RefCell<Option<(i32, String)>> = RefCell::new(None);

    for line in reader.lines() {
        // I gave up handling all of the log file edge cases and
        // just log things that couldn't be parsed - Should all
        // be spam.
        let line = line?;
        let parsed = parse_log_line(&line, &url_parser);
        if parsed.is_err() {
            error!("Can't parse a log line: {}", &line);
            continue;
        }
        if let Some(p) = parsed.unwrap() {
            println!("Found matching line with URL/ID: {}", &p.article_id_or_url);
            // Check if the user agent is in the ignored list:
            let is_ignored_ua = &config
                .ignored_uas
                .iter()
                .any(|ua| p.client_ua.to_lowercase().contains(ua));
            if *is_ignored_ua {
                debug!("Ignoring line due to User Agent {}", &p.client_ua);
                continue;
            }

            // Can we parse the id as an i32?
            // First check whether the last inserted ArticleStat matches that ID
            let article_id = p.article_id_or_url.parse::<i32>().ok().or_else(|| {
                debug!("Found possible article URL");
                // I don't remember why my utility function requires
                // a mutable string but it does.
                if let Some(art) = last_article.borrow().as_ref() {
                    if art.1 == p.article_id_or_url {
                        debug!("Last resolved article has the same URL - id {}", art.0);
                        return Some(art.0);
                    }
                }
                // Get from DB when we don't have it:
                let id_from_db = article_by_url(&pool_main, &p.article_id_or_url)
                    .unwrap_or(None)
                    .map(|a| a.id);

                if id_from_db.is_some() {
                    debug!("Got article ID from database");
                    // Save it in the "cache"
                    // Has to make things extra convoluted to be able to
                    *last_article.get_mut() =
                        Some((id_from_db.unwrap(), p.article_id_or_url.clone()));
                } else {
                    debug!("No article ID was found for the URL");
                }

                id_from_db
            });

            // Do we have that stat in history?
            if article_id.is_some() {
                let entry = StatHistoryItem {
                    article_id: article_id.unwrap(),
                    timestamp: p.timestamp,
                    client_ip: p.client_ip,
                    client_ua: p.client_ua,
                };

                if !stats_history.is_duplicate(&entry)? {
                    let client_ip: IpAddr = entry.client_ip.parse()?;
                    // Get the Geoip info:
                    let geo_info: GeoInfo = iploc.geo_info(client_ip).ok().unwrap_or(GeoInfo {
                        country: String::from(""),
                        region: String::from(""),
                        city: String::from(""),
                    });

                    let article_stat = ArticleStat {
                        id: -1,
                        article_id: entry.article_id,
                        pseudo_ua: pseudonymize(&mut pseudonymizer, &entry.client_ua),
                        pseudo_ip: pseudonymize(&mut pseudonymizer, &entry.client_ip),
                        client_ua: entry.client_ua.clone(),
                        client_ip: ip_utils::extract_first_bytes(&entry.client_ip),
                        date: Some(entry.timestamp),
                        country: geo_info.country,
                        region: geo_info.region,
                        city: geo_info.city,
                    };
                    debug!(
                        "Saving article stat for article ID {}",
                        &article_stat.article_id
                    );

                    // Crash if we can't save the entry in DB
                    let connecton = pool.get()?;
                    insert_article_stat(&connecton, &article_stat)?;

                    // Add to the stats_history:
                    stats_history.add(entry);
                } else {
                    debug!("Found duplicate entry {:?}", &entry);
                }
            }
        }
    }

    Ok(())
}

fn parse_log_line(line: &str, url_parser: &UrlParser) -> Result<Option<ParsedLogLine>> {
    // Thought I could split the lines but they're too weird
    // We have to use a regex. Also this is hyper specific to
    // Nginx, probably.
    lazy_static! {
        // IP, date, URL, status, referrer, user agent
        // Used to not have an optional space in the URL. Added it
        // because some requests do not have any verb and still get
        // logged for some reason. Might be an Nginx thing.
        static ref RE_LOG_LINE: Regex =
            Regex::new(r#"^(\S+?)\s-.+\[(.+?)\]\s"\S{0,5}\s?(\S*?)(\s.+?)?"\s(\d+)\s.+?"(\S*?)"\s"(.*)"$"#).unwrap();
    }

    let captures = RE_LOG_LINE.captures(line);
    if captures.is_none() {
        return Err(eyre!("Log line couldn't be parsed: {}", &line));
    }
    let caps = captures.unwrap();
    // Not sure this can happen but that's my excuse
    // for unleashing an unwrap party:
    if caps.len() < 8 {
        return Err(eyre!("Log line is missing values"));
    }

    // We use the HTTP status to determine if the article
    // "exists" though the site will currently send valid
    // responses with a 404 if the URL is not canonical,
    // sometimes. We'll just ignore that for now.
    let status: u32 = caps[5].parse().unwrap_or(0);
    if status >= 300 || status < 200 {
        return Ok(None);
    }

    // Check if we got an article or short URL in the
    // direct URL or referrer
    let parsed = url_parser
        .parse_url(&caps[3])
        .or_else(|| url_parser.parse_url(&caps[6]));

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
        client_ua: String::from(&caps[7]),
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
    fn url_parser_only_matches_url_or_id() {
        let url = "/breves/119/_payload.json?3e2bb066-5554-4406-bfde-a5f17fb20b86";
        let parser = UrlParser::from("articles", "breves").unwrap();
        let parsed = parser.parse_url(url);

        assert!(parsed.is_none());
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

    #[test]
    fn can_parse_log_line_no_referrer() {
        let line = r###"34.63.167.138 - - [09/Feb/2026:18:56:16 +0100] "GET /articles/javascript_revue_frameworks_2019_2020 HTTP/1.1" 301 162 "" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36""###;
        let url_parser = UrlParser::from("articles", "breves").unwrap();
        let parsed = parse_log_line(line, &url_parser).unwrap();
        assert!(parsed.is_none());
    }

    #[test]
    fn can_parse_log_line_no_verb() {
        // I made that IP address up, sorry if it's yours
        let line1 = r###"91.233.92.2 - - [28/Jan/2026:00:00:46 +0100] "\x03\x00\x00/*\xE0\x00\x00\x00\x00\x00Cookie: mstshash=Administr" 400 150 "-" "-""###;
        let line2 = r###"40.80.203.87 - - [11/Apr/2026:04:56:52 +0200] "MGLNDD_51.255.166.120_443" 400 150 "-" "-""###;
        let url_parser = UrlParser::from("articles", "breves").unwrap();
        let parsed1 = parse_log_line(line1, &url_parser).unwrap();
        let parsed2 = parse_log_line(line2, &url_parser).unwrap();
        assert!(parsed1.is_none() && parsed2.is_none());
    }

    #[test]
    fn can_parse_log_line_empty_url() {
        let line = r###"3.2.241.100 - - [10/Apr/2026:20:44:03 +0200] "" 400 0 "-" "-""###;
        let url_parser = UrlParser::from("articles", "breves").unwrap();
        let parsed = parse_log_line(line, &url_parser).unwrap();
        assert!(parsed.is_none());
    }

    #[test]
    fn can_parse_log_line_no_ua() {
        let line = r###"216.73.12.240 - - [11/Apr/2026:23:33:38 +0200] "GET /robots.txt HTTP/2.0" 200 205 "-" """###;
        let url_parser = UrlParser::from("articles", "breves").unwrap();
        let parsed = parse_log_line(line, &url_parser).unwrap();
        assert!(parsed.is_none());
    }

    #[test]
    fn will_ignore_non_stat_log_line() {
        let line1 = r###"1.1.1.1 - - [20/Jan/2026:11:40:22 +0100] "GET / HTTP/2.0" 200 11152 "-" "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko); compatible; ChatGPT-User/1.0; +https://openai.com/bot""###;
        let line2 = r###"216.73.216.61 - - [09/Apr/2026:22:17:24 +0200] "GET /robots.txt HTTP/2.0" 200 205 "-" "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; ClaudeBot/1.0; +claudebot@anthropic.com)""###;
        let url_parser = UrlParser::from("articles", "breves").unwrap();
        let parsed1 = parse_log_line(line1, &url_parser).unwrap();
        let parsed2 = parse_log_line(line2, &url_parser).unwrap();
        assert!(parsed1.is_none() && parsed2.is_none());
    }
}
