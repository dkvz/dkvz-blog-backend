use serde::{Deserialize, Serialize};

// I'm starting with ultra simple datatypes,
// which is something SQLite fits naturally into.

// These are too simple to be immediately usable
// as JSON after auto-deserialization. I'll have
// to create DTO-like objects like real pros do.

#[derive(Debug, Serialize, Deserialize)]
pub struct Article {
  pub id: i32,
  pub title: String,
  pub article_url: Option<String>,
  pub thumb_image: String,
  pub date: i32,
  pub user_id: i32,
  pub summary: String,
  pub content: Option<String>,
  pub published: i32,
  pub short: i32,
  pub tags: Vec<Tag>
}

// Object I use to fit my "udpate only what's in 
// the request body" agenda.
// We don't allow modifying the "short" status.
#[derive(Debug)]
pub struct ArticleUpdate {
  pub id: i32,
  pub title: Option<String>,
  pub article_url: Option<String>,
  pub thumb_image: Option<String>,
  // The date is a string in update requests:
  pub date: Option<String>,
  pub user_id: Option<i32>,
  pub summary: Option<String>,
  pub content: Option<String>,
  pub published: Option<String>,
  pub tags: Option<Vec<Tag>>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
  pub id: i32,
  pub name: String,
  pub main_tag: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
  pub id: i32,
  pub article_id: i32,
  pub author: String,
  pub comment: String,
  pub date: i32,
  pub client_ip: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArticleStat {
  pub id: i64,
  pub article_id: i32,
  pub pseudo_ua: String,
  pub pseudo_ip: String,
  pub client_ua: String,
  pub client_ip: String,
  pub country: String,
  pub region: String,
  pub city: String,
  pub date: Option<i32>
}
