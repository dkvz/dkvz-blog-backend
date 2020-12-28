// Bunch of enums for query building:
pub enum Order {
  ASC,
  DESC
}

pub struct OrderBy<'a> {
  pub order: Order,
  pub field: &'a str
}

impl<'a> OrderBy<'a> {
  pub fn new(order: Order, field: &'a str) -> Self {
    OrderBy {
      order: order,
      field: field
    }
  }
}

// Decided to put "q_" in front of all args just
// because "where" is a reserved Rust keyword.
// This should probably be a struct with a
// builder pattern.
pub fn select_query_builder(
  q_fields: &Vec<String>, 
  q_from: &Vec<String>,
  q_where: Option<&Vec<String>>,
  q_order: Option<OrderBy>,
  limit: Option<i32>,
  offset: Option<i32>
) -> String {
  let mut query = String::from(
    &format!(
      "SELECT {} FROM {} ",
      &q_fields.join(","),
      &q_from.join(",")
    ) 
  );
  if let Some(wh) = q_where {
    query.push_str(
      &format!(
        "WHERE {} ",
        &wh.join(",")
      ) 
    );
  }
  if let Some(order) = q_order {
    query.push_str(&format!("ORDER BY {} ", order.field));
    query.push_str(
      match order.order {
        Order::ASC => "ASC ",
        Order::DESC => "DESC "
      }
    );
  }
  if let Some(lim) = limit {
    query.push_str(
      &format!(
        "LIMIT {} ",
        lim
      )
    );
    if let Some(off) = offset {
      query.push_str(
        &format!(
          "OFFSET {} ",
          off
        )
      );
    }
  }
  query
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generate_simple_select() {
    let query = select_query_builder(
      &vec!["my_table.name".to_string(), "my_table.value".to_string()], 
      &vec!["my_table".to_string()], 
      None, 
      None, 
      None, 
      None
    );
    // There's supposed to be an extra space at the end and no space between commas:
    let expected = String::from("SELECT my_table.name,my_table.value FROM my_table ");     
    assert_eq!(query, expected);
  }

  #[test]
  fn generate_full_select() {
    let query = select_query_builder(
      &vec!["my_table_1.name".to_string(), "my_table_2.value".to_string()], 
      &vec!["my_table_1".to_string(), "my_table_2".to_string()], 
      Some(&vec!["my_table_1.id = ?".to_string()]), 
      Some(OrderBy::new(Order::DESC, "name")), 
      Some(10), 
      Some(20)
    );
    // There's supposed to be an extra space at the end and no space between commas:
    let expected = String::from(
      "SELECT my_table_1.name,my_table_2.value FROM my_table_1,my_table_2 WHERE my_table_1.id = ? ");     
    assert_eq!(query, expected);
  }
}