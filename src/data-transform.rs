#![allow(dead_code)]
mod config;
mod db;
mod utils;

use fancy_regex::Regex;
use std::env;
use color_eyre::Result;
use eyre::eyre;
use dotenv::dotenv;
use log::{debug, error, info};
use r2d2_sqlite::{self, SqliteConnectionManager};
use getopts::Options;
use crate::db::Pool;
use crate::db::Order;
use crate::config::Config;

// The structure of ths file is horrible I'm so sorry

// Copy pasted this from getopts doc.
fn print_usage(program: &str, opts: Options) {
  let brief = format!("Usage: {} FILE [options]", program);
  print!("{}", opts.usage(&brief));
}

fn run_pre_code_transform(pool: &Pool) -> Result<()> {
  let article_ids = db::all_articles_and_shorts_ids(pool, Order::Asc, false)?;
  for id in article_ids.iter() {

  }
  Ok(())
}

fn transform_pre_code(content: String) -> String {
  // Let's try one of these "negated classes" in regexes
  // We don't want to update any <pre> tag this is already
  // followed by a <code> tag.

  // TODO The regexes should be set as lazy_static items.
  // I have to use one of these cursed negative lookahead 
  // inside of a non-capturing group (?:()).
  // Std regex lib doesn't actually provide these so I had
  // to import some other lib.
  let re_start = Regex::new(r"<pre(.*?)>\s*(?:(?!<code))").unwrap();
  let re_end = Regex::new(r"(?:(?!</code>))\s*</pre>").unwrap();

  let replaced = re_start.replace_all(&content, "<pre$1><code>");
  let replaced = re_end.replace_all(&replaced, "</code></pre$1>");
  return replaced.to_string();
}

/**
 * Binary meant to perform hardcoded database migrations
 */
fn main() -> Result<()> {
  dotenv().ok();
  env_logger::init();

  let args: Vec<String> = env::args().collect();
  let program = args[0].clone();
  let mut opts = Options::new();
  opts.optopt("t", "transform", "Run desired data-transform", "OPERATION");
  opts.optflag("h", "help", "Program usage");
  let opt_matches = opts.parse(args)?;
  if opt_matches.opt_present("h") {
    print_usage(&program, opts);
    return Ok(());
  }

  let config = Config::from_env()
    .expect("Configuration (environment or .env file) is missing");

  // Check operation to run:
  if let Some(operation) = opt_matches.opt_str("t") {
    let manager = SqliteConnectionManager::file(&config.db_path);
    let pool = Pool::new(manager)
      .expect("Database connection failed");
    match operation.as_str() {
      "pre-tags-update" => {
        info!("Start <pre> to <pre><code> transform operation...");

        // Do we need to rebuild the fulltext index? I'd say no.
        // But important to keep in mind for other transforms.

        return Ok(());
      },
      _ => {
        return Err(eyre!("Provided operation doesn't exist for data transform"));
      }
    }
    
  }

  print_usage(&program, opts);  

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn transform_pre_code_happy_path() {
    let code = String::from("<p>test text</p>
      <pre class=\"screen\">const a = 1;
        const b = 3;
        return a + b;
      </pre>
      <p>More text</p>
      <pre class=\"screen\">
      </pre>
    ");
    // Transform will remove line feeds before ending
    // tags, this is expected, code is still correct:
    let expect = String::from("<p>test text</p>
      <pre class=\"screen\"><code>const a = 1;
        const b = 3;
        return a + b;</code></pre>
      <p>More text</p>
      <pre class=\"screen\"><code></code></pre>
    ");
    let result = transform_pre_code(code);
    assert_eq!(expect, result)
  }
}