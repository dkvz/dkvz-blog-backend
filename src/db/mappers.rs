use super::entities::*;
use super::{ArticleSelector};
use rusqlite::{Row, Error};

pub fn map_tag(row: &Row) -> Result<Tag, Error> {
  Ok(Tag {
    id: row.get(0)?,
    name: row.get(1)?,
    main_tag: row.get(2)?
  })
}

pub fn map_articles(
  row: &Row, 
  tags: Vec<Tag>, 
  article_type: ArticleSelector
) -> Result<Article, Error> {
  
}