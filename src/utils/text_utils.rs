use chrono::{Local, TimeZone};

// Stole this from StackOverflow, of course
// https://stackoverflow.com/questions/53570839/quick-function-to-convert-a-strings-first-letter-to-uppercase
pub fn first_letter_to_upper(s1: String) -> String {
  let mut c = s1.chars();
  match c.next() {
    None => String::new(),
    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
  }
}

// Very specific date format the old API is doing: dd/MM/yyyy HH:mm:ssZ
// chrono formatting reference:
// https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html
pub fn timestamp_to_date_string(timestamp: i64) -> String {
  let d = Local.timestamp(timestamp, 0);
  d.format("%d/%m/%Y %k:%M:%S%:z").to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test] 
  fn first_letter_to_upper_on_two_words() {
    let sut = String::from("hello world");
    let expected = String::from("Hello world");
    assert_eq!(first_letter_to_upper(sut), expected);
  }

  #[test]
  fn local_time_formats_as_expected() {
    let timestamp: i64 = 1615150740;
    assert_eq!("07/03/2021 21:59:00+01:00", timestamp_to_date_string(timestamp));
  }

}