/**
 * Generate a certain amount of query placeholders
 */
pub fn generate_query_placeholders(name: &str, count: usize) -> String {
  let mut all_clauses: Vec<String> = Vec::with_capacity(count);
  for i in 0..count {
    all_clauses.push(format!("{} = ?", name));
  }
  all_clauses.join(" AND ")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generate_4_query_placeholders() {
    let name = "tags";
    let count: usize = 4;
    let expected = String::from("tags = ? AND tags = ? AND tags = ? AND tags = ?");     
    assert_eq!(generate_query_placeholders(name, count), expected);
  }
}