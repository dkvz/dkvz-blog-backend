pub mod text_utils;
pub mod ip_utils;
pub mod time_utils;
pub mod serde_utils;

pub fn option_bool_to_i32(value: Option<bool>) -> i32 {
  match value {
    Some(true) => 1,
    _ => 0
  }
}