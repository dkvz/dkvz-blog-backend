/**
* These are integration tests that require a working config
* and ip2location Lite databases for ipv4 and ipv6.
* As such, they're disabled by default.
*/
use eyre::{Result, eyre};

// #[test]
// fn ipv4_iplocation_works() {
//     assert_eq!(2, 2);
// }

fn main() -> Result<()> {
    Err(eyre!("I failed"))
}
