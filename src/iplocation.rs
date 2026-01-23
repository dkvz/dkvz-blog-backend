#![allow(dead_code)]
mod config;
mod db;
mod stats;
mod utils;

use crate::config::Config;
use crate::stats::ip_location::IpLocator;
use color_eyre::Result;
use dotenv::dotenv;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// Pretty much just an integration test for iplocation
// I tried using a [[test]] target but that requires a
// [[lib]] target to work as I want it to so that'll be
// something for later. Probably never.
fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = Config::from_env().expect("Configuration (environment or .env file) is missing");

    let mut iploc = IpLocator::open(&config.iploc_path)?;

    // I hope Google doesn't disappear before my blog backend does
    let test_addr_v4 = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
    let res_v4 = iploc.geo_info(test_addr_v4)?;

    assert_eq!("mountain view", res_v4.city.to_lowercase());
    assert_eq!("california", res_v4.region.to_lowercase());
    assert_eq!("united states of america", res_v4.country.to_lowercase());

    println!("✅ ipv4 tests passed");

    Ok(())
}
