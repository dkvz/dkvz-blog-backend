use serde::{Deserialize, Deserializer};

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
    Some(s) => {
      if s.is_empty() {
        None
      } else {
        Some(s)
      }
    }
    None => None,
  }
}

// Any value that is present is considered Some value, including null.
// See the tests below for the right way to use the deserializer, you
// need specific annotation and a double Option.
pub fn deserialize_null_value<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
  T: Deserialize<'de>,
  D: Deserializer<'de>,
{
  Deserialize::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct FakeArticle {
    // default is mandatory for what we're trying to do,
    // it does NOT work without it.
    #[serde(default, deserialize_with = "deserialize_null_value")]
    thumb_image: Option<Option<String>>,
  }

  #[test]
  fn can_desizeralize_optional_field_as_null() {
    let json_field_present = r#"{"someField": 42, "thumbImage": null}"#;
    let parsed: FakeArticle = serde_json::from_str(json_field_present).unwrap();
    // Finding Some(None) means the field was there but set to null.
    assert_eq!(parsed.thumb_image, Some(None));
  }

  #[test]
  fn can_deserialize_absent_optional_field() {
    let json_field_absent = r#"{"someField": 42}"#;
    let parsed: FakeArticle = serde_json::from_str(json_field_absent).unwrap();
    // Finding None directly means the field was absent.
    assert_eq!(parsed.thumb_image, None);
  }
}
