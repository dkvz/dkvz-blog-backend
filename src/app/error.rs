use actix_web::{
  error::ResponseError,
  HttpResponse
};
//use std::convert::From;
use derive_more::Display;

// I don't really have an elegant way to set the
// content type everywhere...
const ERR_CONTENT_TYPE: &str = "text/plain; charset=utf-8";

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
        HttpResponse::InternalServerError()
          .content_type(ERR_CONTENT_TYPE)
          .body(self.to_string()),
      Error::Forbidden(_) => HttpResponse::Forbidden()
        .content_type(ERR_CONTENT_TYPE)
        .body(self.to_string()),
      Error::NotFound(_) => HttpResponse::NotFound()
        .content_type(ERR_CONTENT_TYPE)
        .body(self.to_string()),
      Error::BadRequest(_) => HttpResponse::BadRequest()
        .content_type(ERR_CONTENT_TYPE)
        .body(self.to_string())
    }
  }
}

// Helper to map database errors (could be replaced with
// a From trait but then I'd need custom database errors)
// This is also where I could decide to panic on DB errors.
// TODO: I should have a custom error type for the DB 
// module and a From trait to convert it into an 
// InternalServerError from this module, would make the 
// code shorter in handlers.rs.
pub fn map_db_error<E: ToString>(err: E) -> Error {
  Error::DatabaseError(err.to_string())
}