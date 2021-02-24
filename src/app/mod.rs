use actix_web::{middleware, web, App, HttpServer, HttpResponse};
use r2d2_sqlite::{self, SqliteConnectionManager};
use color_eyre::Result;
use eyre::{WrapErr, eyre};
use log::{debug, error, info};
use std::net::{IpAddr, Ipv4Addr};
// I think we have to add crate here because
// of the other crate named "config" that we
// use as a dependency.
use crate::config::Config;
use crate::db::Pool;
use crate::stats::{StatsService, BaseArticleStat};


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
    &config.iploc_path
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
      .wrap(middleware::Logger::default())
      .configure(base_endpoints_config)
  })
  .bind(&config.bind_address)?
  .run()
  .await
  .context("Start Actix web server")

}


// TODO Declare route config function:
fn base_endpoints_config(cfg: &mut web::ServiceConfig) {
  cfg.route("/", web::get().to(
    |app_state: web::Data<AppState>| 
      {
        match app_state.stats_service.insert_article_stats(
          BaseArticleStat { 
            article_id: 120,
            client_ip: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            client_ua: "Firefox 28".to_string()
          }
        ) {
          Ok(_) => HttpResponse::Ok().body("Worked. I think."),
          Err(e) => HttpResponse::Ok().body(format!("Error: {}", e))
        }
      }
  ))
  .route("/test", web::get().to(|| {
    info!("Is this working?");
    HttpResponse::Ok().body("Hello from test!")
  }));
}