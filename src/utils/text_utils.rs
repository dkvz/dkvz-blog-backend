use lazy_static::lazy_static;
use regex::{Regex, Captures};
use std::borrow::Cow;

// Stole this from StackOverflow, of course
// https://stackoverflow.com/questions/53570839/quick-function-to-convert-a-strings-first-letter-to-uppercase
pub fn first_letter_to_upper(s1: String) -> String {
  let mut c = s1.chars();
  match c.next() {
    None => String::new(),
    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
  }
}

// Thought about using a library for this, then found out the 
// library was just replacing <, > and & (which I don't need
// to replace in my case) so uh... Yeah let's do it ourselves.
// This might copy the whole string twice, but I'm willing to
// make the sacrifice at this point.
pub fn escape_html<T: AsRef<str>>(s: T) -> String {
  s.as_ref()
    .replace("<", "&lt;")
    .replace(">", "&gt;")
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

// Experimenting with Cow and regretting it here.
// This time around I find relative links by assuming
// they start with a leading "/", if they don't, they
// won't get replaced.
pub fn relative_links_to_absolute<'a>(
  source: &'a str, 
  base_url: &str
) -> Cow<'a, str> {
  // I was using this in Java but lookahead and lookbehind
  // aren't supposed for Rust regexes:
  // "(src=\"|href=\")(?!https?://)/?(.*?)\""

  // The really weird (?i) thing isn't a capture group,
  // it just enables "case insensitive" mode. Yeah I'm
  // surprised too.
  lazy_static! {
    static ref REL_LINK_REGEX: Regex = Regex::new(
      "(?i)(src=\"|href=\")/(.*?)\""
    ).unwrap();
  }

  // That wouldve been too easy:
  //REL_LINK_REGEX.replace_all(source, &format!("$1{}/$2\"", base_url))
  REL_LINK_REGEX.replace_all(
    source,
    |caps: &Captures| {
      format!("{}{}/{}\"", &caps[1], base_url, &caps[2])
    }
  )
}

// More basic version of the previous function that I use
// to generate absolute thumb images, among other things.
pub fn single_link_to_absolute(
  source: impl AsRef<str>,
  base_url: impl AsRef<str>
) -> Option<String> {
  if !source.as_ref().contains("://") {
    Some(format!(
      "{}{}", 
      base_url.as_ref(), 
      source.as_ref()
    ))
  } else {
    None
  }
}

// At some point I found out the truncate method on Strings
// is actually very unsafe as it can make everything panic
// if cutting an unfinished unicode character.
// So we need the horror that unfolds down there.
// Careful that "maxsize" is still in bytes here, check out
// my test below.
// Stole the code from here: 
// https://gist.github.com/dginev/f6da5e94335d545e0a7b
pub fn truncate_utf8(input : &mut String, maxsize: usize) {
  let mut utf8_maxsize = input.len();
  if utf8_maxsize >= maxsize {
    { let mut char_iter = input.char_indices();
    while utf8_maxsize >= maxsize {
      utf8_maxsize = match char_iter.next_back() {
        Some((index, _)) => index,
        _ => 0
      };
    } } // Extra {} wrap to limit the immutable borrow of char_indices()
    input.truncate(utf8_maxsize);
  }
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

  // I know this is akin to testing the html_escape 
  // library but I need to know if it does what I'm 
  // expecting.
  #[test]
  fn escape_html_fits_my_xml_needs() {
    let sut = "<p>Test&nbsp;String &lt;br&gt;</p>";
    assert_eq!(
      "&lt;p&gt;Test&nbsp;String &lt;br&gt;&lt;/p&gt;",
      escape_html(sut)
    );
  }

  #[test]
  fn relative_links_to_absolute_converted() {
    // I'm adding an uppercase SRC in there too.
    let sut = "Example\nText \
      <a href=\"/article/some_url\" target=\"_blank\">\
      some link</a>\n\
      <img SRC=\"/stuff/test.png\" />";
    let expected = "Example\nText \
      <a href=\"https://dkvz.eu/article/some_url\" target=\"_blank\">\
      some link</a>\n\
      <img SRC=\"https://dkvz.eu/stuff/test.png\" />";

    assert_eq!(expected, relative_links_to_absolute(sut, "https://dkvz.eu"));
  }

  #[test]
  fn relative_links_to_absolute_does_not_convert_absolute() {
    let sut = "<a href=\"https://en.wikipedia.org/wiki/Trousers\">\
      awesome article</a>Some more text";

    assert_eq!(sut, relative_links_to_absolute(sut, "https://dkvz.eu"));
  }

  #[test]
  fn truncate_utf8_simple() {
    let mut sut = String::from("just a string éé that's a little too long");
    truncate_utf8(&mut sut, 17);
    assert_eq!(
      "just a string é",
      sut
    );
  }

  #[test]
  fn single_link_to_absolute_returns_none() {
    let sut = "https://wikipedia.org/something/something";
    assert_eq!(
      None,
      single_link_to_absolute(sut, "blahblahbluh")
    );
  }

}