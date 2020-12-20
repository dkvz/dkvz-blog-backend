use rusqlite::{Statement, params, NO_PARAMS};
mod entities;
use eyre::WrapErr;
use color_eyre::Result;
use entities::*;

// Type alias to make function signatures much clearer:
pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

/**
 * I'll do all the DB stuff in a non-async way first.
 */

// TODO I need to make a generic query function and 
// refactor everything.

pub fn all_tags(
  pool: &Pool
) -> Result<Vec<Tag>> {
  // Do the reference counting thingand get a connection
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(
    "SELECT id, name, main_tag FROM tags ORDER BY name ASC"
  )?;
  stmt.query_map(NO_PARAMS, |row| {
    Ok(Tag {
      id: row.get(0)?,
      name: row.get(1)?,
      main_tag: row.get(2)?
    })
  })
  .and_then(Iterator::collect)
  .context("Querying for tags")
}

pub fn comment_count (
  pool: &Pool,
  article_id: i32
) -> Result<i32> {
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(
    "SELECT count(*) FROM comments WHERE article_id = ?"
  )?;
  let count: i32 = stmt.query_row(params![article_id], |row| row.get(0))?;
  Ok(count)
}