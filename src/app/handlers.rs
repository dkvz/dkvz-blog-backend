use actix_web::{
  web, 
  HttpServer, 
  HttpResponse, 
  Result
};
use std::convert::From;
use crate::db::entities::*;
use crate::db::{
  all_tags,
  article_by_id,
  article_by_url
};
use super::dtos::*;
use super::error::Error;
use super::AppState;

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
  match all_tags(&app_state.pool) {
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
  path: web::Path<(String,)>
) -> Result<HttpResponse, Error> {
  let article_url = path.into_inner().0;
  // Check if we got an article ID:
  let article: Option<Article> = match article_url.parse::<i32>() {
    // Fetch article by id:
    Ok(article_id) => article_by_id(&app_state.pool, article_id),
    // Fetch article by URL:
    Err(_) => article_by_url(&app_state.pool, &article_url),
  }.map_err(|e| Error::DatabaseError(e.to_string()))?;
  // Send a 404 if there are no articles:
  match article {
    Some(a) => Ok(HttpResponse::Ok().json(ArticleDto::from(a))),
    None => Err(Error::NotFound("Article does not exist".to_string()))
  }
}