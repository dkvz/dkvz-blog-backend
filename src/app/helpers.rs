use actix_web::{
  HttpRequest
};
use std::net::IpAddr;
use std::str::FromStr;

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
  req.connection_info().realip_remote_addr()
    .map(|ip| {
      // Convert the result into an option:
      IpAddr::from_str(ip)
        .ok()
    })
    // I'm getting an Option of an Option of IpAddr here 
    // so I just have to unwrap one level.
    // My brain is dying.
    .unwrap_or(None)
}