use actix_web::{middleware, web, App, HttpServer, HttpResponse};
use r2d2_sqlite::{self, SqliteConnectionManager};
use color_eyre::Result;
use eyre::{WrapErr, eyre};
use log::{debug, error, info};
use rate_limiter::BasicRateLimiter;
use article_import::ImportService;
use std::sync::RwLock;
// I think we have to add crate here because
// of the other crate named "config" that we
// use as a dependency.
use crate::config::Config;
use crate::db::Pool;
use crate::stats::StatsService;
mod handlers;
mod dtos;
mod error;
mod helpers;
mod rate_limiter;
mod article_import;
mod guards;

// IP addresses allowed to make special calls (like the /rss one).
// Should probably be in the config instead.
pub const ALLOWED_IP_ADDRESSES: [&'static str; 2] = 
  ["127.0.0.1", "::1"];

// Declare app state struct:
pub struct AppState {
  pub pool: Pool,
  pub stats_service: StatsService,
  pub rate_limiter: RwLock<BasicRateLimiter>,
  pub import_service: ImportService
}

// This shouldn't be that weird I'm sorry. These functions
// could be moved elsewhere to not be directly in AppState.
impl AppState {

  pub fn check_rate_limit(&self) -> bool {
    let (needs_update, is_locked) = self.rate_limiter_needs_update();
    if needs_update {
      // Get a lock on the rate limiter:
      match self.rate_limiter.write() {
        Ok(mut rl) => return rl.update(),
        Err(e) => {
          error!("Could not get a write handle on the \
          rate limiter, SHOULD NEVER HAPPEN - {}", e);
        }
      }
    }
    return is_locked
  }

  // Returns tuple: "needs update" first, then the current
  // is_locked value.
  fn rate_limiter_needs_update(&self) -> (bool, bool) {
    match self.rate_limiter.read() {
      Ok(rl) => (
        !rl.is_locked() || (rl.is_locked() && rl.is_expired()),
        rl.is_locked()
      ),
      Err(e) => {
        // I decided to ignore possible weird rate limiter lock
        // errors which should never happen.
        error!("Could not get a read handle on the rate limiter - \
          SHOULD NEVER HAPPEN - {}", e);
        (false, false)
      }
    }
  }

}

// Function to start the server.
// Has to be async because there should be a .await at the end.
// I'm not certain it's even allowed to put this all here as 
// there's this #[actix_web::main] decorator thingy that I'm 
// going to use in main.rs.
pub async fn run() -> Result<()> {
  let config = Config::from_env()
    .expect("Configuration (environment or .env file) is missing");
  debug!("Current config: {:?}", config);
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


  // Declare the import service, crash immediately 
  // if import directory is not writable:
  let import_service = ImportService::from(&config.import_path)
    .expect("Fatal: import directory is not writable");

  let app_state = web::Data::new(
    AppState {
      pool,
      stats_service,
      import_service,
      rate_limiter: RwLock::new(
        BasicRateLimiter::new(
          config.rl_max_requests, 
          config.rl_max_requests_time, 
          config.rl_block_duration
        )
      )
    }
  );
  
  HttpServer::new(move|| {
    App::new()
      .app_data(app_state.clone())
      .app_data(web::PathConfig::default().error_handler(|_, _| {
        // No idea how this works but it does:
        actix_web::error::ErrorBadRequest("Invalid path arguments")
      }))
      .app_data(web::QueryConfig::default().error_handler(|_, _| {
        actix_web::error::ErrorBadRequest("Invalid query string arguments")
      }))
      .wrap(middleware::Logger::default())
      .configure(base_endpoints_config)
      .default_service(web::route().to(handlers::not_found))
  })
  .bind(&config.bind_address)?
  .run()
  .await
  .context("Start Actix web server")

}

// Route configuration:
fn base_endpoints_config(cfg: &mut web::ServiceConfig) {
  // Create the guard that cause protected endpoints to respond with a 404
  // when the client IP address isn't allowed.
  let ip_guard = guards::IPRestrictedGuard::new(&ALLOWED_IP_ADDRESSES);

  cfg.route("/", web::get().to(handlers::index))
    .route("/tags", web::get().to(handlers::tags))
    .route("/article/{articleUrl}", web::get().to(handlers::article))
    .route("/articles-starting-from/{start}", web::get().to(handlers::articles_starting_from))
    .route("/shorts-starting-from/{start}", web::get().to(handlers::shorts_starting_from))
    .route("/comments", web::post().to(handlers::post_comment))
    .route("/last-comment", web::get().to(handlers::last_comment))
    .route("/import-articles", web::get().to(handlers::import_article))
    .route("/articles/search", web::post().to(handlers::search_articles))
    .route("/rss", web::get().guard(ip_guard).to(handlers::rss));
}