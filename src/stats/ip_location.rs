// IP Location module using ip2Location.
use color_eyre::Result;
use eyre::eyre;
use ip2location::DB;
use std::net::IpAddr;

// Expecting a specific set of Geo data:
#[derive(Debug)]
pub struct GeoInfo {
    pub country: String,
    pub region: String,
    pub city: String,
}

pub struct IpLocator {
    dbv4: DB,
    dbv6: Option<DB>,
}

impl IpLocator {
    pub fn open(filename_v4: &str, filename_v6: &Option<String>) -> Result<IpLocator> {
        let dbv4 = DB::from_file(filename_v4).map_err(|_| eyre!("Error opening ip2location DB"))?;
        let dbv6 = match filename_v6 {
            Some(f) => DB::from_file(f).ok(),
            None => None,
        };

        Ok(Self { dbv6, dbv4 })
    }

    // Add function to request GeoInfo.
    // The "db" object has to be mutable.
    pub fn geo_info(&mut self, ip: IpAddr) -> Result<GeoInfo> {
        match self.dbv4.ip_lookup(ip) {
            Ok(record) => {
                let country = match record.country {
                    Some(country) => remove_dash(country.long_name),
                    None => String::new(),
                };
                let region = record.region.unwrap_or(String::new());
                let city = record.city.unwrap_or(String::new());
                // For some reasons, ip2location seems to add dashes
                // for certain IP addresses - Removing it for empty
                // string
                Ok(GeoInfo {
                    country,
                    region: remove_dash(region),
                    city: remove_dash(city),
                })
            }
            Err(_) => Err(eyre!("Error with IP location")),
        }
    }
}

fn remove_dash(value: String) -> String {
    if value == "-" { String::new() } else { value }
}
