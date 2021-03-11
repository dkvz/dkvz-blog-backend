use actix_web::{
  web, 
  HttpServer, 
  HttpResponse, 
  HttpRequest, 
  Result
};
use std::convert::From;
use crate::db::entities::*;
use crate::db;
use crate::stats::{BaseArticleStat, StatsService};
use log::{error, info};
use super::dtos::*;
use super::error::Error;
use super::AppState;
use super::helpers;

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
pub async fn article<'a>(
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