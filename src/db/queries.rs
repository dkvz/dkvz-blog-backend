// My QUERY BUILDING system ended up being a lot more
// convoluted than I thought, but it works. I mean it 
// will work at some point.

use std::fmt;

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

// I'm going to use this to provide
// a way to stitch the WHERE clause
// arguments using either only AND or 
// only OR. Yes this is kinda weird 
// I'm sorry.
// It's a private implementation detail 
// though.
// Also at this point I'm aware this 
// is a boolean.
enum WhereClauseGlue {
  And,
  Or
}

pub struct OrderBy {
  pub order: Order,
  pub field: String
}

impl OrderBy {
  pub fn new(order: Order, field: String) -> Self {
    OrderBy {
      order,
      field
    }
  }
}

// Decided to use the "builder pattern"
// they talk about in Rust docs for 
// query building.
// The "q_" in front of field names is 
// just because "where" is a reserved
// keyword in Rust.
pub struct Query {
  q_fields: Vec<String>, 
  q_type: QueryType,
  q_where: Option<Vec<String>>,
  where_glue: Option<WhereClauseGlue>,
  q_order: Option<OrderBy>,
  limit: Option<i32>,
  offset: Option<i32>,
}

impl Query {
  
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
    self.where_glue = Some(WhereClauseGlue::And);
    self
  }

  pub fn where_or(mut self, q_where: Vec<String>) -> Self {
    self.q_where = Some(q_where);
    self.where_glue = Some(WhereClauseGlue::Or);
    self
  }

  pub fn order(mut self, order: OrderBy) -> Self {
    self.q_order = Some(order);
    self
  }

  pub fn limit(mut self, limit: i32) -> Self {
    self.limit = Some(limit);
    self
  }

  pub fn offset(mut self, offset: i32) -> Self {
    self.offset = Some(offset);
    self
  }

  // Get the "WHERE" part of the query
  fn query_where_str(&self) -> Option<String> {
    if let Some(wh) = &self.q_where {
      format!(
        "WHERE {} ",
        &wh.join(
          
        )
      )
    }
    String::new()
  }

}

// Creating the query string will be done by 
// implementing the Display trait, which will
// all implement ToString.
impl fmt::Display for Query {

  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // The start part of the query can't be the 
    // Display trait for QueryType because we 
    // need access to "q_fields".
    let mut query = match &self.q_type {
      QueryType::Select(q_from) => format!(
        "SELECT {} FROM {} ",
        &self.q_fields.join(","),
        &q_from.join(",")
      ),
      QueryType::Delete(table) => format!(
        "DELETE FROM {} ",
        &table
      ),
      QueryType::Insert(table) => format!(
        "INSERT INTO {} ({}) VALUES ",
        &table,
        &self.q_fields.join(",")
      ),
      QueryType::Update(table)=> format!(
        "UPDATE {} SET ",
        &table
      ),
    };

    write!(
      f, "{}", query
    )
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
  let mut query = format!(
    "SELECT {} FROM {} ",
    &q_fields.join(","),
    &q_from.join(",")
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
      Some(OrderBy::new(Order::Desc, "name".to_string())), 
      Some(10), 
      Some(20)
    );
    // There's supposed to be an extra space at the end and no space between commas:
    let expected = String::from(
      "SELECT my_table_1.name,my_table_2.value FROM my_table_1,my_table_2 WHERE my_table_1.id = ? ");     
    assert_eq!(query, expected);
  }
}