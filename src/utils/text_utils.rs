// Stole this from StackOverflow, of course
// https://stackoverflow.com/questions/53570839/quick-function-to-convert-a-strings-first-letter-to-uppercase
pub fn first_letter_to_upper(s1: String) -> String {
  let mut c = s1.chars();
  match c.next() {
    None => String::new(),
    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
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

}