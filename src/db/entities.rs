use serde::{Deserialize, Serialize};

// I'm starting with ultra simple datatypes,
// which is something SQLite fits naturally into.

#[derive(Serialize, Deserialize)]
pub struct Article {
  pub id: i32,
  pub title: String,
  pub article_url: String,
  pub thumb_image: String,
  pub date: i32,
  pub user_id: i32,
  pub summary: String,
  pub content: String,
  pub published: i32,
  pub short: i32
}

#[derive(Serialize, Deserialize)]
pub struct Tag {
  pub id: i32,
  pub name: String,
  pub main_tag: i32
}

