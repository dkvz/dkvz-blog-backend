use rusqlite::{Statement, NO_PARAMS};
mod entities;
use entities::*;

// Type alias to make function signatures much clearer:
type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

// Trying it as non-async first:
pub fn all_tags(
  pool: &Pool
) -> Result<Vec<Tag>> {
  // Do the reference counting thingand get a connection
  let conn = pool.clone().get()?;
  let stmt = conn.prepare(
    "SELECT id, name, main_tag FROM tags ORDER BY name ASC"
  )?;
  stmt.query_map(NO_PARAMS, |row| {
    Ok(Tag {
      id: row.get(0)?,
      name: row.get(1)?,
      main_tag: row.get(2)?
    })
  })
  .and_then(Iterator::collect())
}