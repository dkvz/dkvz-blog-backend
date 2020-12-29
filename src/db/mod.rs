use rusqlite::{Statement, params, NO_PARAMS, Row, ToSql, OptionalExtension};
mod entities;
mod mappers;
mod helpers;
mod queries;
use eyre::{WrapErr, eyre};
use color_eyre::Result;
use entities::*;
// Re-exporting the query building enums and structs:
pub use queries::{Order, OrderBy};
use queries::{Query, QueryType};
use helpers::generate_where_placeholders;
use mappers::map_tag;

/**
 * I'll do all the DB stuff in a non-async way first.
 */

// Type alias to make function signatures much clearer:
pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

// Some enums used in DB functions:
pub enum ArticleSelector {
  SHORT,
  ARTICLE,
  ALL
}

// Stole most of the signature from the rustqlite doc.
// Careful to use a later version of the crate, 
// Google takes you to old versions of the doc.
fn select_many<T, P, F>(
  pool: &Pool, 
  query: &str, 
  params: P, 
  mapper: F
) -> Result<Vec<T>> 
  where
    P: IntoIterator,
    P::Item: ToSql,
    F: FnMut(&Row<'_>) -> Result<T, rusqlite::Error>,
{
  // Do the reference counting thingand get a connection
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(query)?;
  stmt.query_map(params, mapper)
    .and_then(Iterator::collect)
    .context("Generic select_many query")
}

fn select_one<T, P, F>(
  pool: &Pool, 
  query: &str, 
  params: P, 
  mapper: F 
) -> Result<Option<T>>
  where
  P: IntoIterator,
  P::Item: ToSql,
  F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
// Do the reference counting thing and get a connection
let conn = pool.clone().get()?;
let mut stmt = conn.prepare(query)?;
// .optional() won't work unless we import the 
// OptionalExtension trait from rusqlite.
stmt.query_row(params, mapper)
  .optional()
  .context("Generic select_once query")
}

/*
------------------------------------------------------
Data access functions
------------------------------------------------------
*/

pub fn all_tags(
  pool: &Pool
) -> Result<Vec<Tag>> {
  select_many(
    pool, 
    "SELECT id, name, main_tag FROM tags ORDER BY name ASC", 
    NO_PARAMS, 
    map_tag
  )
}

pub fn comment_count (
  pool: &Pool,
  article_id: i32
) -> Result<i32> {
  let count_opt: Option<i32> = select_one(
    pool,
    "SELECT count(*) FROM comments WHERE article_id = ?",
    params![article_id],
    |row| row.get(0)
  )?;
  // The generic function supports having optional values,
  // But the count query here should never just not give
  // any value.
  match count_opt {
    Some(count) => Ok(count),
    None => Err(eyre!("A count query returned no value")) 
  }
}

pub fn get_tags_for_article(
  pool: &Pool,
  article_id: i32
) -> Result<Vec<Tag>> {
  select_many(
    pool, 
    "SELECT tags.id, tags.name, tags.main_tag 
    FROM article_tags, tags WHERE 
    article_tags.article_id = ? 
    AND article_tags.tag_id = tags.id", 
    params![article_id], 
    map_tag
  )
}

// Trying to upgrade from the horrible mess I had in the Java app
// for article retrieval.
// The same function has to be able to retrieve ALL articles too.
// Used to have an infamous string buffer query building system,
// I upgraded to a struct with a builder pattern.
// That struct isn't actually easy to use but it makes the code
// easy to read.
pub fn articles_from_to(
  article_selector: ArticleSelector,
  start: usize,
  count: usize,
  tags: Option<Vec<String>>,
  order: Order
) -> Result<Vec<Article>> {
  let mut query = String::from(
    "SELECT articles.id, articles.title, articles.article_url, 
    articles.thumb_image, articles.date, articles.user_id, 
    articles.summary, articles.published"
  );
  // Add the article content to the fields list when
  // ArticleSelector is ALL or ARTICLE:
  match article_selector {
    ArticleSelector::ALL | ArticleSelector::ARTICLE => 
      query.push_str(", articles.content"),
    ArticleSelector::SHORT => ()
  }
  query.push_str(" FROM articles ");

  Ok(Vec::new())
}