mod config;
mod db;
use db::{ 
    Pool, 
    all_tags, 
    comment_count, 
    articles_from_to, 
    article_by_url,
    insert_article,
    delete_article,
    last_comment,
    search_published_articles,
    ArticleSelector,
    Order
};
use db::entities::*;
use color_eyre::Result;
use r2d2_sqlite::{self, SqliteConnectionManager};
// I think we have to add crate here because
// of the other crate named "config" that we
// use as a dependency.
use crate::config::Config;

fn main() -> Result<()> {
  let config = Config::from_env()
    .expect("Configuration (environment or .env file) is missing");

  let manager = SqliteConnectionManager::file(&config.db_path);
  let pool = Pool::new(manager)
    .expect("Database connection failed");


  /*
  let tags = all_tags(&pool)?;
  let count = comment_count(&pool, 110)?;
  let articles = articles_from_to(&pool, ArticleSelector::Short, 0, 10, None, Order::Desc)?;
  for article in &articles {
      println!("{} - {}", article.id, article.title);
  }
  let id = "ma_soiree_sur_marketplace";
  let article = article_by_url(&pool, id)?;
  match article {
      Some(article) => println!("Article: {:?}", article),
      None => println!("No article found for id {}", &id)
  }
  println!("Found config: {:?}", config);
  println!("Found tags: {}", tags.len());
  println!("Comment count for article 110: {}", count);
  */

  /*
  let some_tag = Tag {
    id: 2,
    name: "Whatever".to_string(),
    main_tag: 1
  };
  let mut article = Article {
    id: -1,
    article_url: Some("cool_test_article".to_string()),
    content: Some("Cool test article".to_string()),
    title: "Another cool article".to_string(),
    date: 1611056903,
    published: 1,
    short: 0,
    summary: "This is the article summary".to_string(),
    thumb_image: "/some/image.png".to_string(),
    user_id: 1,
    tags: vec![some_tag]
  };
  let result = insert_article(&pool, &mut article)?;
  println!("Inserted article with id {}", result);
  println!("Article after insert: {:?}", article);
  */

  /* // Snippet to delete articles
  let result = delete_article(&pool, 121)?;
  println!("Affected {} rows", result); */

  /* // Snippet for last comment
  let last_comment: Option<Comment> = last_comment(&pool)?;
  match last_comment {
    Some(com) => println!("{:?}", com),
    None => println!("No comment found")
  }*/

  let result = search_published_articles(
    &pool,
    &["slip", "prout"]
  )?;
  for article in &result {
    println!("{:?}", article);
  }

  Ok(())
}
