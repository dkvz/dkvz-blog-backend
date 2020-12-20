use rusqlite::{Statement, params, NO_PARAMS, Row, ToSql};
mod entities;
mod mappers;
use eyre::{WrapErr, eyre};
use color_eyre::Result;
use entities::*;
use mappers::map_tag;

// Type alias to make function signatures much clearer:
pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

/**
 * I'll do all the DB stuff in a non-async way first.
 */

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
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(
    "SELECT count(*) FROM comments WHERE article_id = ?"
  )?;
  let count: i32 = stmt.query_row(
    params![article_id], 
    |row| row.get(0)
  )?;
  Ok(count)
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