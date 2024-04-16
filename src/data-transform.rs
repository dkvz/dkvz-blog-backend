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
use lazy_static::lazy_static;
use crate::db::Pool;
use crate::db::entities::*;
use crate::db::Order;
use crate::config::Config;

// The structure of ths file is horrible I'm so sorry

// WARNING: Data transform operations neither ask for 
// confirmation nor create a backup of the DB for you.
// You should DEFINITELY create a backup before though.

// Copy pasted this from getopts doc.
fn print_usage(program: &str, opts: Options) {
  let brief = format!("Usage: {} FILE [options]", program);
  print!("{}", opts.usage(&brief));
}

fn run_pre_tags_update(pool: &Pool) -> Result<()> {
  let article_ids = db::all_articles_and_shorts_ids(pool, Order::Asc, false)?;
  for id in article_ids.iter() {
    // Get the article. We just stop the whole thing immediately in case of error.
    let article = db::article_by_id(pool, *id)?;
    if let Some(a) = article {
      info!("Processing article {} - '{}'", &id, &a.title);
      let article_update = ArticleUpdate::update_content(
        id.clone(),
        transform_pre_code(a.summary),
        transform_pre_code(a.content.unwrap_or(String::from("")))
      );
      // We don't need it but update_article also updates the fulltext index.
      db::udpate_article(pool, &article_update)?;
    }
  }
  Ok(())
}

fn transform_pre_code(content: String) -> String {
  // I have to use one of these cursed negative lookahead 
  // inside of a non-capturing group (?:()).
  // Std regex lib doesn't actually provide these so I had
  // to import some other lib.
  // Lead to a whole bunch of issues making due to me wanting
  // to copy the classes (and other possible arguments) in 
  // <pre> so in the end I made a character class that includes
  // almost everything but ">":
  // <pre(\s*[\w\d\s“'-=]*?)>(?:(?!<code))
  // This whole ordeal feels extremely wonky, hence my many 
  // tests.
  // Ideally we'd need a HTML parser here.
  lazy_static! {
    static ref RE_START: Regex = Regex::new(
      r#"<pre(\s*[\w\d\s"'-=]*?)>(?:(?!\s*<code))"#
    ).unwrap();
    // For that one we need a lookbehind (of course):
    static ref RE_END: Regex = Regex::new(
      r"(?:(?<!<\/code>))\s*<\/pre>"
    ).unwrap();
    //let re_end = Regex::new(r"(?:(?!\s*<\/code>\s*))<\/pre>").unwrap();
    // The regex above sometimes produces double code ending tags,
    // so here's a bonus one to remove them.
    // I'm having a lot of fun.
    static ref RE_DOUBLE_CODE: Regex = Regex::new(
      r"<\/code>\s*<\/code>\s*<\/pre>"
    ).unwrap();
  }

  let replaced = RE_START.replace_all(&content, "<pre$1><code>");
  let replaced = RE_END.replace_all(&replaced, "</code></pre>");
  let replaced = RE_DOUBLE_CODE.replace_all(&replaced, "</code></pre>");
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
        return run_pre_tags_update(&pool);
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

  #[test]
  fn transform_pre_code_no_action_when_code_already_there() {
    let code = String::from("Some text<hr>More text
      <pre><code class=\"javascript\">void(false)</code></pre>
      even more text, modify this one
      <pre>
      // epic code right there
      // More of it
      </pre>
      <b>OK I'm done</b>");
    let expect = String::from("Some text<hr>More text
      <pre><code class=\"javascript\">void(false)</code></pre>
      even more text, modify this one
      <pre><code>
      // epic code right there
      // More of it</code></pre>
      <b>OK I'm done</b>");
    let result = transform_pre_code(code);
    assert_eq!(expect, result)
  }

  #[test]
  fn transform_pre_code_space_characters_in_between_no_replace() {
    let code = String::from("<pre>
    
        <code>
        // Code here
        </code>


        </pre>
        <p>Text that follows</p>
      ");
    let expect = String::from("<pre>
    
        <code>
        // Code here
        </code></pre>
        <p>Text that follows</p>
      ");
    let result = transform_pre_code(code);
    assert_eq!(expect, result)
  }

}