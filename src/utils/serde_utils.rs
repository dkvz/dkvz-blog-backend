//use serde::{Deserialize, Deserializer};

// Copy pasted from here: https://github.com/serde-rs/serde/issues/1425
// To be used with annotation:
// #[serde(deserialize_with = "serde_utils::empty_string_is_none")]
// Mostly useful for DTOs.
/*pub fn empty_string_is_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
  D: Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  if s.is_empty() {
      Ok(None)
  } else {
      Ok(Some(s))
  }
}*/

// Previous thingy wasn't actually working.
// I'll be doing empty string to None in the DTO conversion
// using plain old function here:
pub fn empty_string_to_none(value: Option<String>) -> Option<String> {
  match value {
    Some(s) => if s.is_empty() 
      { None } else { Some(s) },
    None => None
  }
}