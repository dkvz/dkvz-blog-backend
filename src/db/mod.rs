use rusqlite::{
  Statement, 
  params, 
  NO_PARAMS, 
  Row, 
  ToSql, 
  OptionalExtension
};
mod entities;
mod mappers;
mod helpers;
mod queries;
use eyre::{WrapErr, eyre};
use std::convert::TryFrom;
use color_eyre::Result;
use entities::*;
// Re-exporting the query building enums and structs:
pub use queries::{Order, OrderBy};
use queries::{Query, QueryType};
use helpers::{generate_where_placeholders, stripped_article_content};
use mappers::{map_tag, map_article, map_count};

/**
 * I'll do all the DB stuff in a non-async way first.
 * For those that do not know my style (lol), I never
 * specify INNER JOIN when that type of JOIN is used,
 * I always use some "=" in a WHERE clause instead.
 * I also try to avoid using any of the other JOIN 
 * whatsoever.
 */

// Type alias to make function signatures much clearer:
pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

// Some enums used in DB functions:
pub enum ArticleSelector {
  Short,
  Article,
  All
}

// Stole most of the signature from the rustqlite doc.
// Careful to use a later version of the crate, 
// Google takes you to old versions of the doc.
fn select_many<T, P, F>(
  pool: &Pool, 
  query: &str, 
  params: P, 
  mapper: F
) -> Result<Vec<T>> 
  where
    P: IntoIterator,
    P::Item: ToSql,
    F: FnMut(&Row<'_>) -> Result<T, rusqlite::Error>,
{
  // Do the reference counting thingand get a connection
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(query)?;
  stmt.query_map(params, mapper)
    .and_then(Iterator::collect)
    .context("Generic select_many query")
}

fn select_one<T, P, F>(
  pool: &Pool, 
  query: &str, 
  params: P, 
  mapper: F 
) -> Result<Option<T>>
  where
  P: IntoIterator,
  P::Item: ToSql,
  F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
// Do the reference counting thing and get a connection
let conn = pool.clone().get()?;
let mut stmt = conn.prepare(query)?;
// .optional() won't work unless we import the 
// OptionalExtension trait from rusqlite.
stmt.query_row(params, mapper)
  .optional()
  .context("Generic select_one query")
}

fn select_count<P>(
  pool: &Pool, 
  query: &str, 
  params: P, 
) -> Result<i64> 
  where
  P: IntoIterator,
  P::Item: ToSql,
{
  let count = select_one(pool, query, params, map_count)?
    .unwrap_or(0);
  Ok(count)
}

/*
------------------------------------------------------
Reusable mappers and query functions
------------------------------------------------------
*/

fn full_article_mapper(
  pool: &Pool,
  row: &Row
) -> Result<Article, rusqlite::Error> {
  let article_id = row.get(0)?;
  let short: i32 = row.get(8)?;
  // Due to how I wrote the mapper function,
  // I have to use the "selector" All for articles
  // or the content is ignored.
  let article_type: ArticleSelector = 
    if short == 0 { ArticleSelector::All }
    else { ArticleSelector::Short };
  // Get the tags:
  map_article(
    row, 
    tags_for_article(pool, article_id)
      .map_err(|_| rusqlite::Error::InvalidQuery)?, 
    &article_type
  )
}

fn insert_article_tag(
  pool: &Pool,
  tag: &Tag,
  article_id: i32
) -> Result<usize> {
  let query = Query::new(
    QueryType::Insert { 
      table: "article_tags",
      fields: &["article_id, tag_id"], 
      values: None 
    }
  ).to_string();
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(&query)?;
  stmt.execute(params![article_id, tag.id])
    .context("Insert tag for article")
}

fn delete_article_tag(
  pool: &Pool,
  tag_id: i32,
  article_id: i32
) -> Result<usize> {
  let query = Query::new(
    QueryType::Delete { table: "article_tags" }
  )
    .where_and(&["tag_id = ?", "article_id = ?"])
    .to_string();
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(&query)?;
  stmt.execute(params![tag_id, article_id])
    .context("Delete tag from article")
}

fn insert_article_fulltext(
  pool: &Pool,
  article: &Article
) -> Result<usize> {
  let query = Query::new(
    QueryType::Insert { 
      table: "articles_ft",
      fields: &["id", "title", "content"], 
      values: None 
    }
  ).to_string();
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(&query)?;
  stmt.execute(
    params![
      article.id, 
      article.title, 
      stripped_article_content(&article)
    ]
  ).context("Insert fulltext data for article")
}

fn update_article_fulltext(
  pool: &Pool,
  article: &Article
) -> Result<usize> {
  let query = Query::new(
    QueryType::Update { 
      table: "articles_ft",
      fields: &["title = ?", "content = ?"]
    }
  )
    .where_clause("id = ?")
    .to_string();
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(&query)?;
  stmt.execute(
    params![ 
      article.title, 
      stripped_article_content(&article),
      article.id
    ]
  ).context("Update fulltext data for article")
}

// Yes I know this looks very similar to the previous
// function. Sometimes code repetition is alright guys 
// (by which I mean ME).
fn delete_article_fulltest(
  pool: &Pool,
  article_id: i32
) -> Result<usize> {
  let query = Query::new(
    QueryType::Delete { table: "articles_ft" }
  )
    .where_clause("id = ?")
    .to_string();
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(&query)?;
  stmt.execute(params![article_id])
    .context("Delete fulltext data for article")
}

/*
------------------------------------------------------
Data access functions
------------------------------------------------------
*/

pub fn all_tags(
  pool: &Pool
) -> Result<Vec<Tag>> {
  select_many(
    pool, 
    "SELECT id, name, main_tag FROM tags ORDER BY name ASC", 
    NO_PARAMS, 
    map_tag
  )
}

pub fn comment_count(
  pool: &Pool,
  article_id: i32
) -> Result<i64> {
  /*let count_opt: Option<i32> = select_one(
    pool,
    "SELECT count(*) FROM comments WHERE article_id = ?",
    params![article_id],
    |row| row.get(0)
  )?;*/
  // The generic function supports having optional values,
  // But the count query here should never just not give
  // any value.
  /*match count_opt {
    Some(count) => Ok(count),
    None => Err(eyre!("A count query returned no value")) 
  }*/
  select_count(
    pool,
    "SELECT count(*) FROM comments WHERE article_id = ?",
    params![article_id]
  )
}

pub fn tags_for_article(
  pool: &Pool,
  article_id: i32
) -> Result<Vec<Tag>> {
  select_many(
    pool, 
    "SELECT tags.id, tags.name, tags.main_tag 
    FROM article_tags, tags WHERE 
    article_tags.article_id = ? 
    AND article_tags.tag_id = tags.id", 
    params![article_id], 
    map_tag
  )
}

// Trying to upgrade from the horrible mess I had in the Java app
// for article retrieval.
// The same function has to be able to retrieve ALL articles too.
// Used to have an infamous string buffer query building system,
// I upgraded to a struct with a builder pattern.
// That struct isn't actually easy to use but it makes the code
// easy to read.
// Hardcoded to only be able to get published articles.
pub fn articles_from_to(
  pool: &Pool,
  article_selector: ArticleSelector,
  start: usize,
  count: usize,
  tags: Option<Vec<&str>>,
  order: Order
) -> Result<Vec<Article>> {
  let mut from = vec!["articles"];
  let mut fields = vec![
    "articles.id",
    "articles.title", 
    "articles.article_url", 
    "articles.thumb_image",
    "articles.date",
    "articles.user_id", 
    "articles.summary",
    "articles.published",
    "articles.short"
  ];
  // Add the article content to the fields list when
  // ArticleSelector is ALL or SHORT (we don't add it
  // to ARTICLES because these have huge content):
  if let ArticleSelector::All | ArticleSelector::Short = article_selector { 
    fields.push("articles.content");
  }
  let mut q_where = vec!["articles.published = 1"];
  // Kinda redundant, "if let" above is almost the same check
  match article_selector {
    ArticleSelector::Article => q_where.push("articles.short = 0"),
    ArticleSelector::Short => q_where.push("articles.short = 1"),
    _ => ()
  }
  // Have to declare this here as it has to live as long as the
  // q_where vector does.
  // I could just use a copy and fix this but uh... Yeah.
  let placeholders: String;
  if let Some(tag_list) = &tags {
    if tag_list.len() > 0 {
      // Append actually drains ("move" is more accurate) the 
      // provided vector, so it needs a mutable one.
      from.append(&mut vec!["article_tags", "tags"]);
      q_where.push("(tags.id = article_tags.tag_id AND \
        article_tags.article_id = articles.id)");
      placeholders = generate_where_placeholders("tags.name", tag_list.len());
      q_where.push(
        placeholders.as_str()
      );
    }
  }
  // Build the query. I order by id and not by date for 
  // performance reasons. I don't know, it's historical.
  let query = Query::new(
    QueryType::Select { 
      from: &from,
      fields: &fields
    }
  )
    .where_and(&q_where)
    .order(OrderBy::new(order, "articles.id"))
    .limit(count)
    .offset(start)
    .to_string();

  // haven't thought of something more "optimal" than
  // providing an empty vector.
  let params = match tags {
    Some(ts) => ts,
    None => Vec::new()
  };

  select_many(
    pool, 
    query.as_str(), 
    params, 
    |row| {
      let article_id = row.get(0)?;
      // We always get the tags, even though I never use them on "shorts",
      // I might do someday.
      // My "error handling" is subpar, mapping Eyre error into one of the
      // parameter-less member of rusqlite::Error.
      map_article(
        row, 
        tags_for_article(pool, article_id)
          .map_err(|_| rusqlite::Error::InvalidQuery)?, 
        &article_selector
      )
    }
  )
  
}

pub fn article_by_id(
  pool: &Pool,
  id: i32
) -> Result<Option<Article>> {
  select_one(
    pool,
    "SELECT id, title, article_url, thumb_image, date, user_id, \
    summary, published, short, content FROM articles WHERE id = ?",
    params![id],
    |row| {
      full_article_mapper(&pool, &row)
    }
  )
}

pub fn article_by_url(
  pool: &Pool,
  url: &str
) -> Result<Option<Article>> {
  select_one(
    pool,
    "SELECT id, title, article_url, thumb_image, date, user_id, \
    summary, published, short, content FROM articles \
    WHERE article_url = ?",
    params![url],
    |row| {
      full_article_mapper(&pool, &row)
    }
  )
}

// Returns a result with the ID of the inserted article when
// successful. It's an i64 because that's what the SQLite
// lib is providing.
pub fn insert_article(
  pool: &Pool,
  article: &Article
) -> Result<i64> {
  // We expect the date to have been set by the caller,
  // which has the responsibility to put current date 
  // when needed.
  // As always, I'm not using transactions because 
  // nobody got time for that but it would be better
  // as a single transaction.
  let query = Query::new(
    QueryType::Insert {
      table: "articles",
      fields: &[
        "title", 
        "article_url", 
        "thumb_image", 
        "date", 
        "user_id", 
        "summary", 
        "content", 
        "published", 
        "short"
      ],
      values: None
    }
  ).to_string();
  let conn = pool.clone().get()?;

  let mut stmt = conn.prepare(&query)?;
  stmt.execute(
    params![
      article.title,
      article.article_url,
      article.thumb_image,
      article.date,
      article.user_id,
      article.summary,
      article.content,
      article.published,
      article.short
    ]
  )?;
  let article_id: i64 = conn.last_insert_rowid();
  // Insert tags. This creates an extra connection in 
  // the called function. Oh well...
  for tag in article.tags.iter() {
    // Could be an error if the id is too large to fit inside i32.
    // Shouldn't happen though - But I should replace all the i32s 
    // for i64s at some point.
    insert_article_tag(&pool, tag, i32::try_from(article_id)?)?;
  }
  // Insert fulltext data:
  insert_article_fulltext(&pool, &article)?;
  Ok(article_id)
}