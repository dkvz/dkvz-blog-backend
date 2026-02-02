use color_eyre::Result;
use dkvz_blog_backend::app;
use dotenv::dotenv;
use std::env;

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv().ok();
    if env::var("RUST_LOG").ok().is_none() {
        unsafe {
            env::set_var("RUST_LOG", "info,actix_web=info");
        }
    }
    env_logger::init();
    // Defautl BIND_ADDRESS is not set in the config module.
    /*if env::var("BIND_ADDRESS").ok().is_none() {
      env::set_var("BIND_ADDRESS", "127.0.0.1:8080");
    }*/

    app::run().await
}
