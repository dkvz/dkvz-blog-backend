use super::entities::*;
use super::{ArticleSelector};
use rusqlite::{Row, Error};

// Decided to use something larger than i32 for...
// reasons. I don't know I liked the challenge.
// Then I found out they only provide From traits
// for i64 because of the minus sign or something.
pub fn map_count(row: &Row) -> Result<i64, Error> {
  Ok(row.get(0)?)
}

pub fn map_tag(row: &Row) -> Result<Tag, Error> {
  Ok(Tag {
    id: row.get(0)?,
    name: row.get(1)?,
    main_tag: row.get(2)?
  })
}

pub fn map_article(
  row: &Row, 
  tags: Vec<Tag>, 
  article_type: &ArticleSelector,
  author: String,
  comments_count: i64
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
      tags,
      author,
      comments_count
    }
  )
}

// Some comment queries do not ask for client_ip.
pub fn map_comment(row: &Row) -> Result<Comment, Error> {
  let client_ip: Option<String> = match row.get(5) {
    Ok(ip) => Some(ip),
    Err(_) => None
  };
  Ok(
    Comment {
      id: row.get(0)?,
      article_id: row.get(1)?,
      author: row.get(2)?,
      comment: row.get(3)?,
      date: row.get(4)?,
      client_ip
    }
  )
}

pub fn map_search_result(
  row: &Row
) -> Result<Article, Error> {
  Ok(
    Article {
      id: row.get(0)?,
      title: row.get(1)?,
      //article_url: Some(row.get(2)?),
      article_url: row.get(2)?,
      short: row.get(3)?,
      date: row.get(4)?,
      user_id: row.get(5)?,
      summary: row.get(6)?,
      content: None,
      published: 1,
      thumb_image: None,
      tags: Vec::new(),
      comments_count: 0,
      author: row.get(7)?
    }
  )
}