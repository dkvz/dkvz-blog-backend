mod config;
mod db;
mod stats;
mod utils;
mod app;
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
use std::env;
use std::net::{IpAddr, Ipv4Addr};
use stats::{StatsService, BaseArticleStat};
use color_eyre::Result;
use r2d2_sqlite::{self, SqliteConnectionManager};
// I think we have to add crate here because
// of the other crate named "config" that we
// use as a dependency.
use crate::config::Config;

#[actix_web::main]
async fn main() -> Result<()> {
  if env::var("RUST_LOG").ok().is_none() {
    env::set_var("RUST_LOG", "info,actix_web=info");
  }
  env_logger::init();

  app::run().await
}

/*fn main() -> Result<()> {
  let config = Config::from_env()
    .expect("Configuration (environment or .env file) is missing");

  let manager = SqliteConnectionManager::file(&config.db_path);
  let pool = Pool::new(manager)
    .expect("Database connection failed");

  /*
  let tags = all_tags(&pool)?;
  let count = comment_count(&pool, 110)?;
  let articles = articles_from_to(&pool, Arlet manager = SqliteConnectionManager::file(&config.db_path);
  let pool = Pool::new(manager)
    .expect("Database connection failed");ticleSelector::Short, 0, 10, None, Order::Desc)?;
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

  /* // Snippet for search
  let result = search_published_articles(
    &pool,
    &["fleur", "pantalon"]
  )?;
  for article in &result {
    println!("{:?}", article);
  }*/

  /*// StatsService snippet
  let stats_service = StatsService::open(&pool, &config.wordlist_path, &config.iploc_path)?;
  stats_service.insert_article_stats(
    BaseArticleStat { 
      article_id: 120,
      client_ip: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
      client_ua: "Firefox 28".to_string()
    }
  )?;*/

  Ok(())
}*/
