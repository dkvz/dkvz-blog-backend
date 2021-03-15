use actix_web::{
  web, 
  HttpServer, 
  HttpResponse, 
  HttpRequest, 
  Result
};
use std::convert::{From, TryInto};
use crate::db::entities::*;
use crate::db;
use crate::stats::{BaseArticleStat, StatsService};
use serde::{Deserialize, Serialize};
use log::{error, info};
use super::dtos::*;
use super::error::Error;
use super::AppState;
use super::helpers;

// Few constants I don't know where to put. They 
// don't really qualify for the config file:
const MAX_ARTICLES: usize = 30;

/* --- Request body or query objects --- */
// These have to be public.
#[derive(Serialize, Deserialize)]
pub struct ArticlesQuery {
  pub max: Option<usize>,
  pub tags: Option<String>,
  pub order: Option<String>
}
/* --- End request body or query objects --- */

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
  }.map_err(|e| Error::DatabaseError(e.to_string()))?;
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
    .map_err(|e| Error::DatabaseError(e.to_string()))?
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
    ).map_err(|e| Error::DatabaseError(e.to_string()))?;

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