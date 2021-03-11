use actix_web::{
  HttpRequest
};
use std::net::IpAddr;
use std::str::FromStr;
use regex::Regex;
use lazy_static::lazy_static;

// Extracting Actix header values is kinda convoluted.
// They check for an error in the header value not 
// being convertable to string because of uh... 
// invalid characters or something.
pub fn header_value(req: &HttpRequest) -> String {
  req.headers().get("user-agent")
    .map(|h| String::from(h.to_str().unwrap_or("")))
    .unwrap_or(String::new())
}

// It's technically possible to get no IP address from 
// the Actix ConnectionInfo, but I have made it so that
// the stats service absolutely expects an IP address.
pub fn real_ip_addr(req: &HttpRequest) -> Option<IpAddr> {
  // Since there's no way to define a const that uses
  // the heap, we need that weird lazy_static crate.
  // Why isn't this built into the language? Probably
  // they have reasons.
  // The goal of the regex is to remove the port part 
  // from the "IP address" that Actix gives us, which 
  // may or may not have a port part.
  lazy_static! {
    static ref PORT_REGEX: Regex = Regex::new(
      r"(.+):\d+$"
    ).unwrap();
  }

  req.connection_info().realip_remote_addr()
    .map(|ip| {
      // Convert the result into an option:
      IpAddr::from_str(&PORT_REGEX.replace(ip, "$1"))
        .ok()
    })
    // I'm getting an Option of an Option of IpAddr here 
    // so I just have to unwrap one level.
    // My brain is dying.
    .unwrap_or(None)
}