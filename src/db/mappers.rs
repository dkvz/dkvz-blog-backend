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
  article_type: &ArticleSelector
) -> Result<Article, Error> {
  // Field order:
  /*
  "articles.id",
  "articles.title", 
  "articles.article_url", 
  "articles.thumb_image",
  "articles.date",
  "articles.user_id", 
  "articles.summary",
  "articles.published",
  "articles.short",
  "articles.content"
  */
  let (content, article_url): (Option<String>, Option<String>) = 
    match article_type {
      ArticleSelector::All => 
        (Some(row.get(9)?), Some(row.get(2)?)),
      ArticleSelector::Short => 
        (Some(row.get(9)?), None),
      ArticleSelector::Article => 
        (None, Some(row.get(2)?)),
    };
  Ok(
    Article {
      id: row.get(0)?,
      title: row.get(1)?,
      article_url,
      thumb_image: row.get(3)?,
      date: row.get(4)?,
      user_id: row.get(5)?,
      summary: row.get(6)?,
      published: row.get(7)?,
      content,
      short: row.get(8)?,
      tags
    }
  )
}