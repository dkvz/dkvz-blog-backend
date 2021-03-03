use actix_web::{
  error::ResponseError,
  HttpResponse
};
//use std::convert::From;
use derive_more::Display;

// Not sure if it's a good idea to call it "Error" 
// but uh... Yeah I don't know.
// I could use 
// #[display(fmt = "Internal server error: {}", _0)]
// To display the full message, but I don't want it
// to show up to random internet people, the full
// error output should only appear in logs.
#[derive(Debug, Display)]
pub enum Error {
  #[display(fmt = "Internal Server Error")]
  InternalServerError(String),
  #[display(fmt = "Database Error")]
  DatabaseError(String),
  #[display(fmt = "Forbidden: {}", _0)]
  Forbidden(String),
  #[display(fmt = "Not Found: {}", _0)]
  NotFound(String),
  #[display(fmt = "Bad Request (check request params)")]
  BadRequest(String)
}

// I'm using plain text for error responses because it's
// easy and the old API was doing it too. A nice TODO 
// would be to use JSON instead.
impl ResponseError for Error {
  fn error_response(&self) -> HttpResponse {
    match self {
      Error::InternalServerError(_) | Error::DatabaseError(_) => 
        HttpResponse::InternalServerError().body(self.to_string()),
      Error::Forbidden(_) => HttpResponse::Forbidden().body(self.to_string()),
      Error::NotFound(_) => HttpResponse::NotFound().body(self.to_string()),
      Error::BadRequest(_) => HttpResponse::BadRequest().body(self.to_string())
    }
  }
}

// Could declare a bunch of From traits here except I
// don't have other custom errors yet. I could use one
// for data access errors instead of Report from eyre.