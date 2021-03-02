use actix_web::{
  web, 
  HttpServer, 
  HttpResponse, 
  Result
};
use crate::db::{
  all_tags
};
use super::AppState;

pub async fn index() -> HttpResponse {
  HttpResponse::Ok().body("Nothing here")
}

// I'm using the Result from actix_web for this.
pub async fn tags(
  app_state: web::Data<AppState>
) -> Result<HttpResponse> {
  match all_tags(&app_state.pool) {
    Ok(tags) => Ok(HttpResponse::Ok().json(tags)),
    Err(e) => Err(error::Error {})
  }
}