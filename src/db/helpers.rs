use html2text::from_read;
//use super::entities::{Article};
use chrono::prelude::*;

/**
 * Generate a certain amount of query placeholders
 */
pub fn generate_where_placeholders(name: &str, count: usize) -> String {
  let mut all_clauses: Vec<String> = Vec::with_capacity(count);
  for _ in 0..count {
    all_clauses.push(generate_field_equal_qmark(name));
  }
  all_clauses.join(" AND ")
}

pub fn generate_field_equal_qmark(name: &str) -> String {
  format!("{} = ?", name)
}

pub fn strip_html(html: &String) -> String {
  from_read(html.as_bytes(), 70)
}

pub fn stripped_article_content(content: &Option<String>) -> String {
  match content {
    None => String::new(),
    Some(content) => strip_html(content)
  }
}

pub fn current_timestamp() -> i64 {
  Local::now().timestamp()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generate_4_query_placeholders() {
    let name = "tags";
    let count: usize = 4;
    let expected = String::from("tags = ? AND tags = ? AND tags = ? AND tags = ?");     
    assert_eq!(generate_where_placeholders(name, count), expected);
  }
}