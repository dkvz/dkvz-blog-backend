use lazy_static::lazy_static;
use regex::Regex;

// Stole this from StackOverflow, of course
// https://stackoverflow.com/questions/53570839/quick-function-to-convert-a-strings-first-letter-to-uppercase
pub fn first_letter_to_upper(s1: String) -> String {
  let mut c = s1.chars();
  match c.next() {
    None => String::new(),
    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
  }
}

pub fn sanitize_search_terms(
  terms: &Vec<String>, 
  max_search_terms: usize
) -> Vec<String> {
  lazy_static! {
    static ref SEARCH_CLEANUP_REGEX: Regex = Regex::new(
      r"[\[\]\s\$\^%\+-]"
    ).unwrap();
  }

  terms.iter()
    .take(max_search_terms)
    .map(|t| SEARCH_CLEANUP_REGEX.replace_all(t, "").to_string())
    .filter(|t| !t.is_empty())
    .collect()
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
  fn sanitize_search_replaces_illegal_chars() {
    let sut: Vec<String> = vec![
      String::from(" p otato-[po ^w%+-er"),
      String::from("\npotato$   --[power]")
    ];
    let processed = sanitize_search_terms(&sut, 10);
    assert_eq!(processed[0], "potatopower");
    assert_eq!(processed[1], "potatopower");
  }

  #[test]
  fn sanitize_search_enforces_max_terms() {
    let sut: Vec<String> = vec![
      String::from("test1"),
      String::from("test2"),
      String::from("test3"),
      String::from("test4"),
      String::from("test5"),
    ];
    let processed = sanitize_search_terms(&sut, 3);
    assert_eq!(processed.len(), 3);
    assert_eq!(processed[2], "test3");
  }

  #[test]
  fn sanitize_search_removes_empty_strings() {
    let sut: Vec<String> = vec![
      String::from("  $- "),
      String::from(""),
      String::from("  "),
      String::from("\n"),
    ];
    let processed = sanitize_search_terms(&sut, 80);
    assert_eq!(processed.len(), 0);
  }

}