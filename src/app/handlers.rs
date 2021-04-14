use actix_web::{
  web, 
  HttpResponse, 
  HttpRequest, 
  Result
};
use std::convert::{From, TryInto};
use crate::db::entities::*;
use crate::db;
use crate::stats::{BaseArticleStat, StatsService};
use crate::utils::{time_utils, text_utils};
use serde::{Deserialize, Serialize};
use log::{error, info};
use handlebars::Handlebars;
use super::dtos::*;
use super::error::{Error, map_db_error};
use super::AppState;
use super::helpers;

// Module with all the API handler functions.
// Should probably be split into a directory 
// with multiple files grouping handlers together.

// Few constants I don't know where to put. They 
// don't really qualify for the config file:
const MAX_ARTICLES: usize = 30;
const MAX_COMMENTS: usize = 30;
const MAX_COMMENT_LENGTH: usize = 2000;
const MAX_AUTHOR_LENGTH: usize = 70;
// Max length of article content in RSS descriptions:
const MAX_RSS_LENGTH: usize = 2500;
// Max amount of search tersm to process:
const MAX_SEARCH_TERMS: usize = 10;

/* --- Request body or query or form objects --- */
// These have to be public.
#[derive(Serialize, Deserialize)]
pub struct ArticlesQuery {
  pub max: Option<usize>,
  pub tags: Option<String>,
  pub order: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct CommentsQuery {
  pub max: Option<usize>,
  pub start: Option<usize>
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentForm {
  pub comment: String,
  pub author: String,
  pub article_id: Option<i32>,
  pub articleurl: Option<String>
}
/* --- End request body or query or form objects --- */

// This is where you'd choose to panic or not
// when the stats thread is dead for some reason.
// Not a handler, so should probably be moved to 
// helpers.
fn insert_stats(
  article_stat: BaseArticleStat, 
  stats_service: &StatsService
) {
  if let Err(e) = 
    stats_service.insert_article_stats(article_stat) {
      error!("Could not save stats, Stats thread is dead - {}", e);
    }
}

pub async fn index() -> HttpResponse {
  HttpResponse::Ok().body("Nothing here")
}

// Default response when no route matched the request:
pub async fn not_found() -> Result<HttpResponse, Error> {
  Err(Error::NotFound(String::from("Endpoint doesn't exist")))
}

// I'm using the Result from actix_web for this.
// You don't have to use a Result, building the
// right HttpResponse directly works fine too.
// There's also "Responder", which I think is a
// trait?
// Let's use Result everywhere to be consistent,
// see my "error" module for the Error to response
// conversions.
pub async fn tags(
  app_state: web::Data<AppState>
) -> Result<HttpResponse, Error> {
  match db::all_tags(&app_state.pool) {
    Ok(tags) => Ok(HttpResponse::Ok().json(Vec::<TagDto>::from(tags))),
    // I could use something to log the error message
    // somewhere because it won't be shown in browsers
    // for security reasons (see "error" module).
    Err(e) => Err(Error::DatabaseError(e.to_string()))
  }
}

// Path variables have to be in a tuple.
pub async fn article(
  app_state: web::Data<AppState>,
  path: web::Path<(String,)>,
  req: HttpRequest
) -> Result<HttpResponse, Error> {
  let article_url = path.into_inner().0;
  // Check if we got an article ID:
  let article: Option<Article> = match article_url.parse::<i32>() {
    // Fetch article by id:
    Ok(article_id) => db::article_by_id(&app_state.pool, article_id),
    // Fetch article by URL:
    Err(_) => db::article_by_url(&app_state.pool, &article_url),
  }.map_err(map_db_error)?;
  // Send a 404 if there are no articles:
  match article {
    Some(a) => {
      // Save the visit in the stats DB:
      insert_stats(
        BaseArticleStat {
          article_id: a.id,
          client_ua: helpers::header_value(&req),
          client_ip: helpers::real_ip_addr(&req)
        },
        &app_state.stats_service
      );
      
      Ok(HttpResponse::Ok().json(ArticleDto::from(a)))
    },
    None => Err(Error::NotFound("Article does not exist".to_string()))
  }
}

fn articles_or_shorts_starting_from(
  pool: &db::Pool,
  path: web::Path<(usize,)>,
  query: web::Query<ArticlesQuery>,
  article_selector: db::ArticleSelector
) -> Result<HttpResponse, Error> {
  let start = path.into_inner().0;
  let max = query.max.unwrap_or(MAX_ARTICLES);
  let tags: Option<Vec<&str>> = query.tags.as_ref()
    .map(
      |tags_str| tags_str.split(",")
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect()
    )
    .map(|tags_vec: Vec<&str>| if tags_vec.is_empty() { None } else { Some(tags_vec) })
    .unwrap_or(None);
  let order = match &query.order {
    Some(order) => if order.to_lowercase() == "asc" { db::Order::Asc }
      else { db::Order::Desc },
    None => db::Order::Desc
  };
  
  let count: usize = db::article_count(
    pool, 
    &article_selector, 
    &tags
  )
    .map_err(map_db_error)?
    // Convert the i64 to usize:
    .try_into()
    // Handle the case where it can't be converted - Should never happen.
    .map_err(|_| {
      error!("Article count from db::article_count could not be converted to usize");
      Error::InternalServerError(
        String::from("Article count cannot be converted to usize - Should never happen")
      )
    })?;
  // If start is >= count, respond with 404.
  if start >= count {
    Err(Error::NotFound(String::from("No articles found")))
  } else {
    let articles = db::articles_from_to(
      pool, 
      &article_selector, 
      start, 
      max, 
      &tags, 
      order
    ).map_err(map_db_error)?;

    // Might be another way to convert the whole Vec, but I don't know
    // about it.
    let article_dtos: Vec<ArticleDto> = 
      articles.into_iter().map(|a| a.into()).collect();
    Ok(HttpResponse::Ok().json(article_dtos))
  }
}

pub async fn articles_starting_from(
  app_state: web::Data<AppState>,
  path: web::Path<(usize,)>,
  query: web::Query<ArticlesQuery>
) -> Result<HttpResponse, Error> {
  articles_or_shorts_starting_from(
    &app_state.pool, 
    path, 
    query, 
    db::ArticleSelector::Article
  )
}

pub async fn shorts_starting_from(
  app_state: web::Data<AppState>,
  path: web::Path<(usize,)>,
  query: web::Query<ArticlesQuery>
) -> Result<HttpResponse, Error> {
  articles_or_shorts_starting_from(
    &app_state.pool, 
    path, 
    query, 
    db::ArticleSelector::Short
  )
}

pub async fn post_comment(
  app_state: web::Data<AppState>,
  mut comment_form: web::Form<CommentForm>,
  req: HttpRequest
) -> Result<HttpResponse, Error> {
  // Check if we have either article_id or articleurl.
  // article_id has precedence if both are present.
  // Did we use to check if the article exists?
  let article_id = match comment_form.article_id {
    Some(article_id) => article_id,
    None => {
      // Do we have an articleurl?
      match &comment_form.articleurl {
        Some(url) => db::article_id_by_url(&app_state.pool, &url)
          .map_err(map_db_error)?
          .unwrap_or(-1),
        None => -1
      }      
    }
  };
  if article_id <= 0 {
    // Return a BadRequest immediately.
    return Err(Error::BadRequest(
      String::from("Invalid article URL, ID, or no ID provided")
    ));
  }

  // Limit length of body and author, and check if trimmed author 
  // is not empty.
  // I was using truncate at first but it can panic when cutting
  // a multibyte unicode char in half.
  // I actually use a different technique in dtos::RssFeed::add_item.
  //comment_form.comment.truncate(MAX_COMMENT_LENGTH);
  //comment_form.author.truncate(MAX_AUTHOR_LENGTH);
  text_utils::truncate_utf8(&mut comment_form.comment, MAX_COMMENT_LENGTH);
  text_utils::truncate_utf8(&mut comment_form.author, MAX_AUTHOR_LENGTH);

  let author = text_utils::escape_html(comment_form.author.trim());
  if author.is_empty() || comment_form.comment.is_empty() {
    return Err(Error::BadRequest(
      String::from("Author or message body cannot be empty")
    ));
  }

  // This is where I decide to check with my really basic 
  // homemade rate limiter:
  if app_state.check_rate_limit() {
    return Err(Error::TooManyRequests);
  } 
  
  // Note that if we provide an article_id that's >= 0, we don't 
  // actually check if it exists or not. We just update anyway.
  let mut comment = Comment {
    article_id,
    id: -1,
    author,
    comment: text_utils::escape_html(&comment_form.comment),
    date: time_utils::current_timestamp(),
    client_ip: helpers::real_ip_addr(&req)
      .map(|ip| ip.to_string())
  };

  db::insert_comment(&app_state.pool, &mut comment)
    .map_err(|e| {
      error!("Could not insert a comment - {}", e);
      Error::DatabaseError(format!("Failed to insert comment - {}", e))
    })?;

  Ok(HttpResponse::Ok().json(CommentDto::from(comment)))
}

pub async fn last_comment(
  app_state: web::Data<AppState>
) -> Result<HttpResponse, Error> {
  let comm: Option<Comment> = db::last_comment(
    &app_state.pool
  ).map_err(map_db_error)?;
  match comm {
    Some(comment) => Ok(HttpResponse::Ok().json(CommentDto::from(comment))),
    None => Err(Error::NotFound("No comment found".to_string()))
  }
}

// We're using a lock present in app_state to make sure only one 
// import takes place at a given time.
// I think it works. lol.
pub async fn import_article(
  app_state: web::Data<AppState>
) -> HttpResponse {
  match app_state.import_service
    .import_articles(&app_state.pool)
    .await {
      Ok(statuses) => HttpResponse::Ok().json(statuses),
      Err(status) => HttpResponse::Forbidden().json(status)
    }
}

// The search endpoint shares the same rate limiter as the post
// comment one. That same rate_limiter should be a guard or a
// middleware too. It should be a toto item somwhere.
pub async fn search_articles(
  app_state: web::Data<AppState>,
  search_body: web::Json<SearchBody>
) -> Result<HttpResponse, Error> {
  // Do we need to sanitize the terms?
  // They're passed as prepared statement params, but we should
  // probably still remove some special chars.
  // I think we should remove spaces at the very least.
  // Actually, anything considered a space character as for 
  // regexes should be removed (e.g. line feeds should too).
  // Weird invalid regex I was using for Java: [+*$%\\s]
  // I should probably allow "*" but remove "^".

  // First, check rate limiting:
  if app_state.check_rate_limit() {
    return Err(Error::TooManyRequests);
  }

  let sanitized = text_utils::sanitize_search_terms(
    &search_body.include, 
    MAX_SEARCH_TERMS
  );
  // Test that we still got search terms after sanitization!
  if sanitized.is_empty() {
    // It's not actually an error, just return nothing:
    Ok(HttpResponse::Ok().json(Vec::<String>::new()))
  } else {
    let articles = db::search_published_articles(
      &app_state.pool, 
      &sanitized[..]
    ).map_err(map_db_error)?;
    // There is a max number of results per query fixed
    // in the DB function (supposedly at 15).
    Ok(
      HttpResponse::Ok().json(
       articles
        .into_iter()
        .map(Into::into)
        .collect::<Vec<SearchResult>>()
      )
    )
  }  
}

// Because the endpoint is beyond a guard that restricts
// access to a list of IP addresses, I don't rate limit
// or lock anything during requests for the RSS file.
// A cronjob is doing it once or twice a day on my server.
pub async fn rss(
  app_state: web::Data<AppState>,
  hb: web::Data<Handlebars<'_>>
) -> Result<HttpResponse, Error> {
  // In the examples they use the json! macro to create
  // the data to give to handlebars. But it can be anything
  // that implements Serialize from Serde. I created a struct
  // in the dtos module to serve as the full RSS data model.
  let mut data = RssFeed::new(&app_state.site_info, MAX_RSS_LENGTH);

  // Get all the articles one by one by first fetching all
  // of their IDs. I'm doing this hoping the articles will
  // get dropped at each iteration, freeing "some" memory.
  // We ignore DB errors here and just output an empty RSS
  // file if an error happened.
  // I don't limit the amount of articles in the feed, this
  // could eventually get too big.
  if let Ok(ids) = db::all_published_article_and_shorts_ids(
    &app_state.pool, 
    db::Order::Desc
  ) {
    for id in ids {
      // Fetch the article:
      if let Ok(Some(article)) = db::article_by_id(&app_state.pool, id) {
        data.add_item(article);
      }
    }
  }

  let body = hb.render("rss", &data)
    .map_err(|e| {
      error!("A template engine error occued when rendering RSS: {}", e);
      Error::InternalServerError("Template engine error".to_string())
    })?;

  Ok(
    HttpResponse::Ok()
    .content_type("application/xml")
    .body(body)
  )
}

pub async fn comments_starting_from(
  app_state: web::Data<AppState>,
  path: web::Path<(String,)>,
  query: web::Query<CommentsQuery>
) -> Result<HttpResponse, Error> {
  let article_url = path.into_inner().0;
  let start = query.start.unwrap_or_default();
  let max = query.max
    .map(|m| if m > 50 { 50 } else { m })
    .unwrap_or(MAX_COMMENTS);

  // Check if we got an article ID or if we need
  // to get it from the database:
  let article_id = match article_url.parse::<i32>() {
    Ok(article_id) => article_id,
    Err(_) => {
      // Try to find the ID in database:
      match db::article_id_by_url(&app_state.pool, &article_url) {
        Ok(Some(id)) => id,
        // I just don't care about errors here.
        _ => -1
      }  
    }
  };

  // Get the comment count for that article:
  let count = db::comment_count(
    &app_state.pool, 
    article_id
  )
    .map_err(map_db_error)?
    // Convert the i64 to usize:
    .try_into()
    // Handle the case where it can't be converted - Should never happen.
    .map_err(|_| {
      error!("Article count from db::article_count could not be converted to usize");
      Error::InternalServerError(
        String::from("Article count cannot be converted to usize - Should never happen")
      )
    })?;
  // If start is >= count, respond with 404.
  if start >= count {
    Err(Error::NotFound(String::from("No comments found")))
  } else {
    let comments: Vec<CommentDto> = db::comments_from_to(
      &app_state.pool, 
      start, 
      max, 
      article_id
    )
      .map_err(map_db_error)?
      .into_iter()
      .map(|c| CommentDto::from(c).remove_article_id())
      .collect();

    Ok(HttpResponse::Ok().json(comments))
  }
}

pub async fn sitemap(
  app_state: web::Data<AppState>,
  hb: web::Data<Handlebars<'_>>
) -> Result<HttpResponse, Error> {

  Err(Error::NotFound("Not implemented yet".to_string()))
}