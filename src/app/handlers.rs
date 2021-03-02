use actix_web::{HttpServer, HttpResponse};

pub async fn index() -> HttpResponse {
  HttpResponse::Ok().body("Nothing here")
}