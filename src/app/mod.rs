use actix_web::{middleware, web, App, HttpServer, HttpResponse};
use r2d2_sqlite::{self, SqliteConnectionManager};
use color_eyre::Result;
use eyre::{WrapErr, eyre};
use log::{debug, error, info};
// I think we have to add crate here because
// of the other crate named "config" that we
// use as a dependency.
use crate::config::Config;
use crate::db::Pool;
use crate::stats::{StatsService};
mod handlers;
mod dtos;
mod error;
mod helpers;

// Declare app state struct:
pub struct AppState {
  pub pool: Pool,
  pub stats_service: StatsService
}

// Function to start the server.
// Has to be async because there should be a .await at the end.
// I'm not certain it's even allowed to put this all here as 
// there's this #[actix_web::main] decorator thingy that I'm 
// going to use in main.rs.
pub async fn run() -> Result<()> {
  let config = Config::from_env()
    .expect("Configuration (environment or .env file) is missing");
  let manager = SqliteConnectionManager::file(&config.db_path);
  let pool = Pool::new(manager)
    .expect("Database connection failed");

  // Declare the StatsService, start its thread
  // It has its own separate database.
  let manager_stats = SqliteConnectionManager::file(&config.stats_db_path);
  let pool_stats = Pool::new(manager_stats)
    .expect("Could not connect to stats database");
  let stats_service = StatsService::open(
    &pool_stats, 
    &config.wordlist_path, 
    &config.iploc_path,
    config.message_queue_size
  )?;

  let app_state = web::Data::new(
    AppState {
      pool,
      stats_service
    }
  );
  
  HttpServer::new(move|| {
    App::new()
      .app_data(app_state.clone())
      .app_data(web::PathConfig::default().error_handler(|_, _| {
        // No idea how this works but it does:
        actix_web::error::ErrorBadRequest("Invalid path and/or query arguments")
      }))
      .wrap(middleware::Logger::default())
      .configure(base_endpoints_config)
      .default_service(
        web::route().to(handlers::not_found)
      )
  })
  .bind(&config.bind_address)?
  .run()
  .await
  .context("Start Actix web server")

}

// Route configuration:
fn base_endpoints_config(cfg: &mut web::ServiceConfig) {
  cfg.route("/", web::get().to(handlers::index))
    .route("/tags", web::get().to(handlers::tags))
    .route("/article/{articleUrl}", web::get().to(handlers::article))
    .route("/articles-starting-from/{start}", web::get().to(handlers::articles_starting_from))
    .route("/shorts-starting-from/{start}", web::get().to(handlers::shorts_starting_from));
}