use serde::{Deserialize, Serialize};

// I'm starting with ultra simple datatypes,
// which is something SQLite fits naturally into.

// These are too simple to be immediately usable
// as JSON after auto-deserialization. I'll have
// to create DTO-like objects like real pros do.

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct Tag {
  pub id: i32,
  pub name: String,
  pub main_tag: i32
}

