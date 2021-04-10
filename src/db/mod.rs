use rusqlite::{
  Statement, 
  params, 
  NO_PARAMS, 
  Row, 
  ToSql, 
  OptionalExtension
};
pub mod entities;
mod mappers;
mod helpers;
mod queries;
use eyre::{WrapErr, eyre};
use log::{info};
use std::convert::TryFrom;
use color_eyre::Result;
use entities::*;
// Re-exporting the query building enums and structs:
pub use queries::{Order, OrderBy};
use queries::{Query, QueryType};
use helpers::{
  generate_where_placeholders, 
  stripped_article_content,
  generate_field_equal_qmark
};
use mappers::{
  map_tag, 
  map_article, 
  map_count, 
  map_comment,
  map_search_result
};
use crate::utils::time_utils::current_timestamp;

/**
 * I'll do all the DB stuff in a non-async way first.
 * For those that do not know my style (lol), I never
 * specify INNER JOIN when that type of JOIN is used,
 * I always use some "=" in a WHERE clause instead.
 * I also try to avoid using any of the other JOIN 
 * whatsoever.
 */

const ANONYMOUS_USERNAME: &'static str = "Anonymous";

// Type alias to make function signatures much clearer:
pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

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
  row: &Row,
  article_type: Option<&ArticleSelector>
) -> Result<Article, rusqlite::Error> {
  let article_id = row.get(0)?;
  let short: i32 = row.get(8)?;
  let user_id: i32 = row.get(5)?;
  // Due to how I wrote the mapper function,
  // I have to use the "selector" All for articles
  // or the content is ignored.
  // At some point I allowed forcing around this 
  // behavior by providing a value into the
  // article_type Option.
  let article_selector: &ArticleSelector = match article_type {
    Some(article_type) => article_type,
    None => if short == 0 { &ArticleSelector::All }
      else { &ArticleSelector::Short }
  };
  // Get the tags, username, and comment count:
  map_article(
    row, 
    tags_for_article(pool, article_id)
      .map_err(|_| rusqlite::Error::InvalidQuery)?, 
    &article_selector,
    username_for_id(pool, user_id)
      .map_err(|_| rusqlite::Error::InvalidQuery)?,
    comment_count(pool, article_id)
      .map_err(|_| rusqlite::Error::InvalidQuery)?
  )
}

// Check if something exists by ID. I chose
// to use "count".
fn entry_exists(
  pool: &Pool,
  query: &str,
  id: i32
) -> Result<bool> {
  let count = select_count(
    pool, 
    query, 
    params![id]
  )?;
  Ok(count == 1)
}

// Trying to reuse connections here.
fn insert_article_tag(
  connection: &Connection,
  tag_id: i32,
  article_id: i32
) -> Result<usize> {
  let query = Query::new(
    QueryType::Insert { 
      table: "article_tags",
      fields: &["article_id", "tag_id"], 
      values: None 
    }
  ).to_string();
  /*let conn = match connection {
    Some(conn) => conn,
    None => &pool.clone().get()?
  };*/
  let mut stmt = connection.prepare(&query)?;
  stmt.execute(params![article_id, tag_id])
    .context("Insert tag for article")
}

fn delete_all_tags_for_article(
  connection: &Connection,
  article_id: i32
) -> Result<usize> {
  let query = Query::new(
    QueryType::Delete { table: "article_tags" }
  )
    .where_clause("article_id = ?")
    .to_string();
  
  let mut stmt = connection.prepare(&query)?;
  stmt.execute(params![article_id])
    .context("Delete tag from article")
}

fn insert_article_fulltext(
  connection: &Connection,
  article: &Article
) -> Result<usize> {
  insert_article_fulltext_by_values(
    &connection, 
    &article.title, 
    &article.content, 
    article.id
  )
}

// Need this to work around not having to create
// some weird trait to work as a generic for 
// "partial" articles.
fn insert_article_fulltext_by_values(
  connection: &Connection,
  title: &String,
  content: &Option<String>,
  article_id: i32
) -> Result<usize> {
  let query = Query::new(
    QueryType::Insert { 
      table: "articles_ft",
      fields: &["id", "title", "content"], 
      values: None 
    }
  ).to_string();
  //let conn = pool.clone().get()?;
  let mut stmt = connection.prepare(&query)?;
  stmt.execute(
    params![
      article_id, 
      title, 
      stripped_article_content(&content)
    ]
  ).context("Insert fulltext data for article")
}

// I can't use Article as argument here because
// the update method uses a different struct 
// entirely, I'd have to implement From and 
// that's useless memory allocation so yeah.
fn update_article_fulltext(
  connection: &Connection,
  article: &ArticleUpdate
) -> Result<usize> {
  // We just return Ok(0) immediately if there's 
  // nothing to update, we don't error for that
  // case.
  match (&article.title, &article.content) {
    (None, None) => return Ok(0),
    _ => ()
  };
  let mut fields: Vec<&str> = Vec::new();
  let mut values: Vec<&dyn ToSql> = Vec::new();
  let (f_title, f_content) = ("title = ?", "content = ?");
  if let Some(title) = &article.title {
    fields.push(f_title);
    values.push(title);
  }
  if let Some(content) = &article.content {
    fields.push(f_content);
    values.push(content);
  }
  values.push(&article.id);
  let query = Query::new(
    QueryType::Update { 
      table: "articles_ft",
      fields: &fields
    }
  )
    .where_clause("id = ?")
    .to_string();
  //let conn = pool.clone().get()?;
  let mut stmt = connection.prepare(&query)?;
  stmt.execute(values)
    .context("Update fulltext data for article")
}

// Yes I know this looks very similar to the previous
// function. Sometimes code repetition is alright guys 
// (by which I mean ME).
fn delete_article_fulltext(
  connection: &Connection,
  article_id: i32
) -> Result<usize> {
  let query = Query::new(
    QueryType::Delete { table: "articles_ft" }
  )
    .where_clause("id = ?")
    .to_string();
  let mut stmt = connection.prepare(&query)?;
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

pub fn username_for_id(
  pool: &Pool, 
  user_id: i32
) -> Result<String> {
  let conn = pool.clone().get()?;
  // The old API was substituting "Anonymous" to possibly invalid/unknown
  // user IDs during database row processing, doing the same here.
  let mut stmt = conn.prepare("SELECT name FROM users WHERE id = ?")?;
  stmt.query_row(
    params![user_id],
    |row| -> Result<String, rusqlite::Error> {
      Ok(row.get(0)?)
    }
  )
    .optional()
    .context(format!("Fetch usename for user_id {}", user_id))?
    // Need to recreate a Result after we unwrapped the Option:
    .map_or(
      Ok(ANONYMOUS_USERNAME.to_string()),
      |username| Ok(username)
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
  article_selector: &ArticleSelector,
  start: usize,
  count: usize,
  tags: &Option<Vec<&str>>,
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
  let params: Vec<&str> = match tags {
    Some(ts) => ts.clone(),
    None => Vec::new()
  };

  select_many(
    pool, 
    query.as_str(), 
    params, 
    |row| {
      full_article_mapper(pool, row, Some(&article_selector))
    }
  )
}

pub fn article_count(
  pool: &Pool,
  article_selector: &ArticleSelector,
  tags: &Option<Vec<&str>>
) -> Result<i64> {
  let mut from = vec!["articles"];
  let mut q_where = vec!["articles.published = 1"];
  // Yes the following lines are a huge copy paste from the function
  // above.
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

  // haven't thought of something more "optimal" than
  // providing an empty vector.
  let params: Vec<&str> = match tags {
    Some(ts) => ts.clone(),
    None => Vec::new()
  };

  let query = Query::new(
    QueryType::Select { 
      from: &from,
      fields: &["count(*)"]
    }
  )
    .where_and(&q_where)
    .to_string();

  select_count(
    &pool, 
    &query, 
    params
  )
}

// I use this to check for article existence
// because fetching the whole article + tags 
// etc is more costly.
pub fn article_id_by_url(
  pool: &Pool,
  url: &str
) -> Result<Option<i32>> {
  select_one(
    pool,
    "SELECT id FROM articles WHERE article_url = ?",
    params![url],
    |row| {
      row.get(0)
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
      full_article_mapper(&pool, &row, None)
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
      full_article_mapper(&pool, &row, None)
    }
  )
}

pub fn article_exists(
  pool: &Pool,
  id: i32
) -> Result<bool> {
  entry_exists(
    pool,
    "SELECT count(*) FROM articles WHERE id = ? LIMIT 1",
    id
  )
}

pub fn tag_exists(
  pool: &Pool,
  id: i32
) -> Result<bool> {
  entry_exists(
    pool,
    "SELECT count(*) FROM tags WHERE id = ? LIMIT 1", 
    id
  )
}

pub fn user_exists(
  pool: &Pool,
  id: i32
) -> Result<bool> {
  entry_exists(
    pool,
    "SELECT count(*) FROM users WHERE id = ? LIMIT 1", 
    id
  )
}

// Returns a result with the ID of the inserted article when
// successful.
pub fn insert_article(
  pool: &Pool,
  article: &mut Article
) -> Result<i32> {
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
  // Could be an error if the id is too large to fit inside i32.
  // Shouldn't happen though - But I should replace all the i32s 
  // for i64s at some point.
  let article_id: i32 = i32::try_from(conn.last_insert_rowid())?;
  // At some point I decided to also modify the struct:
  article.id = article_id;
  // Insert tags. I tried to reuse the current connection because
  // I'm fun like that.
  for tag in article.tags.iter() {
    insert_article_tag(&conn, tag.id, article_id)?;
  }
  // Insert fulltext data:
  insert_article_fulltext(&conn, &article)?;
  Ok(article_id)
}

// Deletes so much stuff it should really be a
// transaction. Oh well...
// Note that it doesn't error if nothing is deleted 
// (e.g. because article doesn't exist),
// just returns Ok(0).
pub fn delete_article(
  pool: &Pool,
  article_id: i32
) -> Result<usize> {
  let conn = pool.clone().get()?;
  // Remove fulltext and tags first:
  delete_article_fulltext(&conn, article_id)?;
  delete_all_tags_for_article(&conn, article_id)?;
  // Delete all comments:
  let q_del_comms = Query::new(
    QueryType::Delete { table: "comments" }
  )
    .where_clause("article_id = ?")
    .to_string();
  let mut stmt = conn.prepare(&q_del_comms)?;
  let parms = params![article_id];
  stmt.execute(parms)?;
  // Remove the actual article, shadowing 
  // previous vars:
  let query = Query::new(
    QueryType::Delete { table: "articles" }
  )
    .where_clause("id = ?")
    .to_string();
  let mut stmt = conn.prepare(&query)?;
  stmt.execute(parms)
    .context("Delete article")
}

// Updating articles is weird in that we check for 
// the presence of fields to update or we don't touch
// them (because the API expects this behavior).
// We don't check if the article exists here, will just
// return Ok(0) if nothing happened.
// Also, was complaining about code repetition before,
// this function put it in a different perspective.
pub fn udpate_article(
  pool: &Pool,
  article: &ArticleUpdate
) -> Result<usize> {
  // Gotta use Strings or I get a whole bunch of
  // temporary values dropped in my evil "if let"
  // mania below.
  let mut fields: Vec<String> = Vec::new();
  // If the ToSql trait is imported, we can put a 
  // whole bunch of different data types in the same
  // vector.
  let mut values: Vec<&dyn ToSql> = Vec::new();
  // Kind of ugly but we do what we can - Time for a 
  // WHOLE BUNCH OF IF LET statements.
  // Could be made cleaner by putting all the option
  // statuses in some list paired with their names
  // and work from there.
  if let Some(title) = &article.title {
    fields.push(generate_field_equal_qmark("title"));
    values.push(title);
  }
  if let Some(article_url) = &article.article_url {
    fields.push(generate_field_equal_qmark("article_url"));
    values.push(article_url);
  }
  // thumb_image is special because it's possible to set
  // it to null to really also set it to null in the 
  // database. We use a double Option for it.
  if let Some(thumb_image) = &article.thumb_image {
    fields.push(generate_field_equal_qmark("thumb_image"));
    match thumb_image {
      Some(image) => values.push(image),
      None => values.push(thumb_image) 
    }
  }
  if let Some(user_id) = &article.user_id {
    fields.push(generate_field_equal_qmark("user_id"));
    values.push(user_id);
  }
  if let Some(summary) = &article.summary {
    fields.push(generate_field_equal_qmark("summary"));
    values.push(summary);
  }
  if let Some(content) = &article.content {
    fields.push(generate_field_equal_qmark("content"));
    values.push(content);
  }
  if let Some(published) = &article.published {
    fields.push(generate_field_equal_qmark("published"));
    values.push(published);
  }
  // Check that there's at least one field OR that tags are present.
  // If not, we return Ok(0) immediately.
  let got_tags: bool = match &article.tags {
    Some(_) => true,
    None => false
  };
  let got_fields = fields.len() > 0;
  match (got_fields, got_tags) {
    (false, false) => Ok(0), // Return immediately, no error
    _ => {
      let conn = pool.clone().get()?;
      let mut result = 0;
      if got_fields {
        // update the article ; Need to transform the Vec of Strings to 
        // an array of &str too.
        let query = Query::new(
          QueryType::Update { 
            table: "articles", 
            fields: &fields.iter().map(|s| s as &str).collect::<Vec<&str>>()
          }
        )
          .where_clause("id = ?")
          // SQLite doesn't allow limit in update and delete statements.
          //.limit(1)
          .to_string();
        // We need the article id in values too:
        values.push(&article.id);
        let mut stmt = conn.prepare(&query)?;
        result = stmt.execute(values)?;
        // Update the fulltext data:
        update_article_fulltext(&conn, &article)?;
      }
      if let Some(tags) = &article.tags {
        // Delete all tags and re-add them all.
        // This is easier than checking what's there or not.
        delete_all_tags_for_article(&conn, article.id)?;
        for tag in tags.iter() {
          insert_article_tag(&conn, tag.id, article.id)?;
          result += 1;
        }
      }
      Ok(result)
    }
  }
}

// Rebuilds the entire fulltext index from the articles table.
pub fn rebuild_fulltext(pool: &Pool) -> Result<usize> {
  // SELECT id, title, content FROM articles WHERE published = 1 ORDER BY id ASC
  let conn = pool.clone().get()?;
  // Delete all the current fulltext info.
  // Doesn't need to be a prepared statement but I use them everywhere
  // anyway for convenience and future-proofing.
  let mut stmt = conn.prepare("DELETE FROM articles_fr")?;
  stmt.execute(NO_PARAMS)?;

  let mut stmt = conn.prepare(
    "SELECT id, title, content FROM articles \
    WHERE published = 1 ORDER BY id ASC"
  )?;
  let mut rows = stmt.query(NO_PARAMS)?;
  let mut i = 0;
  while let Some(row) = rows.next()? {
    let id: i32 = row.get(0)?;
    let title: String = row.get(1)?;
    let content: String = row.get(2)?;
    insert_article_fulltext_by_values(
      &conn, 
      &title, 
      &Some(content),
      id
    )?;
    i += 1;
  }
  Ok(i)
}

// Very similar to insert_article.
pub fn insert_comment(
  pool: &Pool,
  comment: &mut Comment
) -> Result<i32> {
  let query = Query::new(
    QueryType::Insert { 
      table: "comments",
      fields: &[
        "article_id",
        "author", 
        "comment", 
        "date", 
        "client_ip"
      ],
      values: None 
    }
  ).to_string();
  let conn = pool.clone().get()?;
  let mut stmt = conn.prepare(&query)?;
  stmt.execute(
    params![
      comment.article_id,
      comment.author,
      comment.comment,
      comment.date,
      comment.client_ip
    ]
  )?;
  // Could be an error if the id is too large to fit inside i32.
  // Shouldn't happen though - But I should replace all the i32s 
  // for i64s at some point.
  let id: i32 = i32::try_from(conn.last_insert_rowid())?;
  // At some point I decided to also modify the struct:
  comment.id = id;
  Ok(id)
}

pub fn last_comment(pool: &Pool) -> Result<Option<Comment>> {
  select_one(
    &pool,
    "SELECT id, article_id, author, comment, date \
     FROM comments ORDER BY id DESC LIMIT 1",
     NO_PARAMS,
    map_comment
  )
}

pub fn comments_from_to(
  pool: &Pool,
  start: usize,
  count: usize,
  article_id: i32
) -> Result<Vec<Comment>> {
  let query = Query::new(
    QueryType::Select {
      from: &["comments", "articles"],
      fields: &[
        "comments.id",
        "comments.article_id",
        "comments.author",
        "comments.comment",
        "comments.date"
      ]
    }
  )
    .where_and(&[
      "articles.id = ?", 
      "articles.id = comments.article_id"
    ])
    .order(OrderBy::new(Order::Asc, "comments.id"))
    .limit(count)
    .offset(start)
    .to_string();

  select_many(
    pool, 
    query.as_str(), 
    params![article_id], 
    map_comment
  )
}

// Uses SQLite fulltext search.
// WARNING: The API endpoint or whatever is using the DB 
// lib will have to clean the search terms up itself first.
pub fn search_published_articles(
  pool: &Pool,
  terms: &[&str]
) -> Result<Vec<Article>> {
  // Copy pasted the query from the old backend. It's probably suboptimal.
  // As other things are in here.
  let query = "SELECT articles_ft.id, articles_ft.title, \
    articles.article_url, articles.short, articles.date, articles.user_id, \
    snippet(articles_ft, 2, '<b>', '</b>', ' [...] ', 50) AS snippet, users.name \
    FROM articles_ft, articles, users WHERE articles_ft MATCH ? \
    AND articles.id = articles_ft.id AND articles.published = 1 \
    AND articles.user_id = users.id \
    ORDER BY rank LIMIT 15";
  select_many(
    pool,
    query,
    params![terms.join(" ")],
    map_search_result
  )
}

// Since my stats are in another DB file, they should
// receive a completely different "pool".
// The data functions just do the data things, no 
// hashing or whatnot is done here.
pub fn insert_article_stat(
  connection: &Connection,
  //article_stat: &mut ArticleStat
  article_stat: &ArticleStat
) -> Result<usize> {
  let query = Query::new(
    QueryType::Insert { 
      table: "article_stats",
      fields: &[
        "article_id", 
        "pseudo_ua", 
        "pseudo_ip", 
        "country", 
        "region", 
        "city", 
        "client_ua", 
        "client_ip", 
        "date"
      ], 
      values: None
    }
  )
    .to_string();
  //let conn = pool.clone().get()?;
  let mut stmt = connection.prepare(&query)?;
  // Check if there's a date, we need to generate 
  // one otherwise.
  // Gonna use unwrap_or to do that.
  stmt.execute(
    params![
      article_stat.article_id,
      article_stat.pseudo_ua,
      article_stat.pseudo_ip,
      article_stat.country,
      article_stat.region,
      article_stat.city,
      article_stat.client_ua,
      article_stat.client_ip,
      article_stat.date.unwrap_or(current_timestamp())
    ]
  ).context("Insert article stats")
  // This is unsed in a multithreaded context, I'd rather
  // not update the id.
  /*let id = conn.last_insert_rowid();
  article_stat.id = id;
  Ok(id)*/
}
