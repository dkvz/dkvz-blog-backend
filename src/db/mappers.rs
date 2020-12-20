use super::entities::*;
use rusqlite::{Row, Error};

pub fn map_tag(row: &Row) -> Result<Tag, Error> {
  Ok(Tag {
    id: row.get(0)?,
    name: row.get(1)?,
    main_tag: row.get(2)?
  })
}