//use std::net::IpAddr;

/*pub fn extract_first_bytes(ip: IpAddr) -> String {
  // We have to check the type of IP address, then
  // we should be able to convert to fixed sized
  // byte arrays.
  match ip {
    IpAddr::V4(ipv4) => {
      let bytes: [u8; 4] = ipv4.octets();
      bytes[]
    },
    IpAddr::V6(ipv6) => {

    }
  }
}*/

// It's much easier to work with strings in the end.
pub fn extract_first_bytes(ip: &str) -> String {
  // Check if we got dots or ":"
  let bytes: Vec<&str> = ip.split('.').collect();
  if bytes.len() == 4 {
    return bytes[0..3].join(".");
  } else if ip != "::1" {
    // Probably ipv6.
    // TODO: Logic here is flawed because of the possible 
    // presence of "::". Still appears to work fine for
    // what I want to do though (see tests).
    let bytes: Vec<&str> = ip.split(":").collect();
    // I'm going to 3 ":" minimum for the byte removal:
    if bytes.len() > 2 {
      return bytes[0..(bytes.len() - 2)].join(":");
    }
  }
  return String::from(ip);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test] 
  fn ipv4_extract_first_bytes() {
    let sut = "111.12.22.254";
    let expected = String::from("111.12.22");
    let sut2 = "127.0.0.1";
    let expected2 = String::from("127.0.0");
    assert_eq!(extract_first_bytes(sut), expected);
    assert_eq!(extract_first_bytes(sut2), expected2);
  }

  #[test] 
  fn ipv6_extract_first_bytes() {
    let sut = "::1";
    let expected = String::from("::1");
    let sut2 = "2001:0db8:85a3:0000:0000:8a2e:0370:7334";
    let expected2 = String::from("2001:0db8:85a3:0000:0000:8a2e");
    let sut3 = "fe80::9656:d028:8652:66b6";
    let expected3 = String::from("fe80::9656:d028");
    assert_eq!(extract_first_bytes(sut), expected);
    assert_eq!(extract_first_bytes(sut2), expected2);
    assert_eq!(extract_first_bytes(sut3), expected3);
  }

  #[test]
  fn invalid_ipv4_gives_same_value() {
    let sut = "222.82";
    let expected = String::from("222.82");
    assert_eq!(extract_first_bytes(sut), expected);
  }

  #[test]
  fn invalid_address_gives_same_value() {
    let sut = "not an address";
    let expected = String::from("not an address");
    assert_eq!(extract_first_bytes(sut), expected);
  }

}