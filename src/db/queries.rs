// My QUERY BUILDING system ended up being a lot more
// convoluted than I thought, but it works. I mean it 
// will work at some point.

// Bunch of enums for query building:
pub enum Order {
  Asc,
  Desc
}

pub enum QueryType {
  Insert(String),
  Select(Vec<String>),
  Update(String),
  Delete(String)
}

enum WhereClauseGlue {
  And,
  Or
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

// Decided to use the "builder pattern"
// they talk about in Rust docs for 
// query building.
// The "q_" in front of field names is 
// just because "where" is a reserved
// keyword in Rust.
pub struct Query<'a> {
  q_fields: Vec<String>, 
  q_type: QueryType,
  q_where: Option<Vec<String>>,
  where_glue: Option<WhereClauseGlue>,
  q_order: Option<OrderBy<'a>>,
  limit: Option<i32>,
  offset: Option<i32>,
}

impl<'a> Query<'a> {
  
  pub fn new(query_type: QueryType, fields: Vec<String>) -> Self {
    Query {
      q_fields: fields,
      q_type: query_type,
      q_where: None,
      where_glue: None,
      q_order: None,
      limit: None,
      offset: None
    }
  }

  pub fn where_clause(mut self, where_str: String) -> Self {
    self.q_where = Some(vec![where_str]);
    self
  }

  pub fn where_and(mut self, q_where: Vec<String>) -> Self {
    self.q_where = Some(q_where);
    self
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
        Order::Asc => "ASC ",
        Order::Desc => "DESC "
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
      Some(OrderBy::new(Order::Desc, "name")), 
      Some(10), 
      Some(20)
    );
    // There's supposed to be an extra space at the end and no space between commas:
    let expected = String::from(
      "SELECT my_table_1.name,my_table_2.value FROM my_table_1,my_table_2 WHERE my_table_1.id = ? ");     
    assert_eq!(query, expected);
  }
}