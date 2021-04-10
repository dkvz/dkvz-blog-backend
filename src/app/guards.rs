use actix_web::{dev::RequestHead, guard::Guard};
use log::warn;

// A guard will just cause the router to not match the
// route and thus probably show a 404. What I'd need 
// would be a middleware and not a guard. But it does
// the trick anyway.
pub struct IPRestrictedGuard<T: 'static + AsRef<str>> {
  allowed_ip_addresses: &'static [T]
}

impl<T: AsRef<str>> IPRestrictedGuard<T> {
  pub fn new(allowed_ips: &'static [T]) -> Self {
    Self {
      allowed_ip_addresses: allowed_ips
    }
  }
}

impl<T: AsRef<str>> Guard for IPRestrictedGuard<T> {
  fn check(&self, req: &RequestHead) -> bool {
    match req.peer_addr {
      Some(sock_addr) => {
        let addr = sock_addr.ip().to_string();
        if self.allowed_ip_addresses.iter().any(|i| i.as_ref() == addr) {
          true
        } else {
          warn!("IP address {} attempted to reach protected \
            endpoint at {}", addr, req.uri);
          false
        }
      },
      None => false
    }
  }
}
