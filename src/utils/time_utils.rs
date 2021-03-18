use chrono::prelude::*;
use chrono::{Local, TimeZone};

// Very specific date format the old API is doing: dd/MM/yyyy HH:mm:ssZ
// chrono formatting reference:
// https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html
const DATE_FORMAT: &'static str = "%d/%m/%Y %k:%M:%S%:z";

pub fn timestamp_to_date_string(timestamp: i64) -> String {
  let d = Local.timestamp(timestamp, 0);
  d.format(DATE_FORMAT).to_string()
}

pub fn current_timestamp() -> i64 {
  Local::now().timestamp()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn local_time_formats_as_expected() {
    let timestamp: i64 = 1615150740;
    assert_eq!("07/03/2021 21:59:00+01:00", timestamp_to_date_string(timestamp));
  }
}