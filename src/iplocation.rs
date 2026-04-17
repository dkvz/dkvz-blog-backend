#![allow(dead_code)]
use color_eyre::Result;
use dkvz_blog_backend::config::Config;
use dkvz_blog_backend::stats::ip_location::IpLocator;
use dotenv::dotenv;
use std::net::{IpAddr, Ipv4Addr};

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
    let test_addr_v6: IpAddr = "2001:4860:4860::8888".parse().unwrap();
    let res_v4 = iploc.geo_info(test_addr_v4)?;
    let res_v6 = iploc.geo_info(test_addr_v6)?;

    assert_eq!("mountain view", res_v4.city.to_lowercase());
    assert_eq!("california", res_v4.region.to_lowercase());
    assert_eq!("united states of america", res_v4.country.to_lowercase());

    println!("✅ ipv4 tests passed");

    assert_eq!("mountain view", res_v6.city.to_lowercase());
    assert_eq!("california", res_v6.region.to_lowercase());
    assert_eq!("united states of america", res_v6.country.to_lowercase());

    println!("✅ ipv6 tests passed");

    Ok(())
}
