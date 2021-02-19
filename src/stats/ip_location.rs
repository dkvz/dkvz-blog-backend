// IP Location module using ip2Location.
use ip2location::DB;
use std::net::IpAddr;
use color_eyre::Result;
use eyre::{WrapErr, eyre};

// Expecting a specific set of Geo data:
pub struct GeoInfo {
  pub country: String,
  pub region: String,
  pub city: String
}

pub struct IpLocator {
  db: DB
}

impl IpLocator {
  
  pub fn open(filename: &str) -> Result<IpLocator> {
    match DB::from_file(filename) {
      Ok(db) => Ok(Self {
        db
      }),
      Err(_) => Err(
        eyre!("Error opening ip2location DB")
      )
    }
  }

  // Add function to request GeoInfo:
  

}