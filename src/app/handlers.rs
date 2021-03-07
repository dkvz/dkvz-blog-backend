use actix_web::{
  web, 
  HttpServer, 
  HttpResponse, 
  Result
};
use std::convert::From;
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
  path: web::Path<(String,)>
) -> HttpResponse {
  let article_url = path.into_inner().0;
  // Check if we got an article ID:
  match article_url.parse::<i32>() {
    Ok(article_id) => HttpResponse::Ok().body(format!("Found article ID: {}", article_id)),
    Err(_) => HttpResponse::Ok().body(format!("Requested article_url: {}", article_url))
  }
}