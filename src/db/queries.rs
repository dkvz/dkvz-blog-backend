// My QUERY BUILDING system ended up being a lot more
// convoluted than I thought, but it works.
// The weirdest part is how to combine OR and AND in
// where clauses, since it's expecting either ONLY Ors,
// or ONLY ANDs. Which can be hacked by providing strings
// with AND or OR already present in them to the query 
// builder.

// IMPORTANT:
// None of this code is doing any escaping on its own.

use std::fmt;

// Bunch of enums for query building:
pub enum Order {
  Asc,
  Desc
}

// When inert values aren't given, "?" prepared
// statement placeholders are automatically
// generated.
pub enum QueryType<'a> {
  Insert { table: &'a str, values: Option<Vec<&'a str>> },
  Select { from: Vec<&'a str> },
  Update { table: &'a str },
  Delete { table: &'a str }
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
  pub fn new(order: Order, field: &str) -> Self {
    OrderBy {
      order,
      field: String::from(field)
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
  q_fields: Vec<&'a str>, 
  q_type: QueryType<'a>,
  q_where: Option<Vec<&'a str>>,
  where_glue: Option<WhereClauseGlue>,
  q_order: Option<OrderBy>,
  limit: Option<usize>,
  offset: Option<usize>,
}

impl<'a> Query<'a> {
  
  pub fn new(query_type: QueryType<'a>, fields: Vec<&'a str>) -> Self {
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

  pub fn where_clause(mut self, where_str: &'a str) -> Self {
    self.q_where = Some(vec![where_str]);
    self
  }

  pub fn where_and(mut self, q_where: Vec<&'a str>) -> Self {
    self.q_where = Some(q_where);
    self.where_glue = Some(WhereClauseGlue::And);
    self
  }

  pub fn where_or(mut self, q_where: Vec<&'a str>) -> Self {
    self.q_where = Some(q_where);
    self.where_glue = Some(WhereClauseGlue::Or);
    self
  }

  pub fn order(mut self, order: OrderBy) -> Self {
    self.q_order = Some(order);
    self
  }

  pub fn limit(mut self, limit: usize) -> Self {
    self.limit = Some(limit);
    self
  }

  pub fn offset(mut self, offset: usize) -> Self {
    self.offset = Some(offset);
    self
  }

  // Get the last part of the query, 
  // WHERE, ORDER and LIMIT
  // May return an empty string as 
  // these clauses are all optional.
  fn where_order_limit_str(&self) -> String {
    let mut result = match &self.q_where {
      Some(wh) => {
        let glue = self.where_glue.as_ref().unwrap_or(&WhereClauseGlue::And);
        let glue_str = match glue {
          WhereClauseGlue::And => " AND ",
          WhereClauseGlue::Or => " OR "
        };
        format!(
          "WHERE {} ",
          &wh.join(glue_str)
        )
      },
      None => String::new()
    };
    // We don't check if the query type actually
    // allows using ORDER BY.
    if let Some(order) = &self.q_order {
      result.push_str(&format!("ORDER BY {} ", order.field));
      result.push_str(
        match order.order {
          Order::Asc => "ASC ",
          Order::Desc => "DESC "
        }
      );
    }
    if let Some(lim) = &self.limit {
      result.push_str(
        &format!(
          "LIMIT {} ",
          lim
        )
      );
      if let Some(off) = &self.offset {
        result.push_str(
          &format!(
            "OFFSET {} ",
            off
          )
        );
      }
    }
    result
  }

}

// Creating the query string will be done by 
// implementing the Display trait, which will
// all implement ToString.
impl<'a> fmt::Display for Query<'a> {

  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // The start part of the query can't be the 
    // Display trait for QueryType because we 
    // need access to "q_fields".
    match &self.q_type {
      QueryType::Select { from: q_from } => {
        write!(
          f,
          "SELECT {} FROM {} {}",
          &self.q_fields.join(","),
          &q_from.join(","),
          &self.where_order_limit_str()
        )
      },
      QueryType::Delete { table } => write!(
        f,
        "DELETE FROM {} {}",
        &table,
        &self.where_order_limit_str()
      ),
      QueryType::Insert { table, values } => {
        // Check if we got values or fill with
        // prepared statement placeholders:
        let values_str = match values {
          Some(vals) => vals.join(","),
          // Dunno if this is the fastest way
          // but it looks cool.
          None => self.q_fields
            .iter()
            .map(|_| "?")
            .collect::<Vec<&str>>()
            .join(",")
        };
        // I'm adding an extra space because
        // all the other queries have one.
        write!(
          f,
          "INSERT INTO {} ({}) VALUES ({}) ",
          &table,
          &self.q_fields.join(","),
          values_str
        )
      },
      QueryType::Update { table } => write!(
        f,
        "UPDATE {} SET {} {}",
        &table,
        &self.q_fields.join(","),
        &self.where_order_limit_str()
      ),
    }
  }

}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generate_simple_select() {
    let query = Query::new(
      QueryType::Select { from: vec!["my_table"] }, 
      vec!["my_table.name", "my_table.value"]
    ).to_string();
    // There's supposed to be an extra space at the end and no space between commas:
    let expected = String::from("SELECT my_table.name,my_table.value FROM my_table ");     
    assert_eq!(query, expected);
  }

  #[test]
  fn generate_full_select() {
    let query = Query::new(
      QueryType::Select { from: vec!["my_table_1", "my_table_2"] }, 
      vec!["my_table_1.name", "my_table_2.value"]
    )
    .where_and(vec!["my_table_1.id = ?", "my_table.other_id = ?"])
    .order(OrderBy::new(Order::Desc, "name"))
    .limit(10)
    .offset(20)
    .to_string();
    // Thank god there's a way to break long strings but keep them from
    // having line breaks in them.
    let expected = String::from(
      "SELECT my_table_1.name,my_table_2.value FROM my_table_1,my_table_2 \
       WHERE my_table_1.id = ? AND my_table.other_id = ? \
       ORDER BY name DESC LIMIT 10 OFFSET 20 ");
    assert_eq!(query, expected);
  }

  #[test]
  fn generate_insert_w_placeholders() {
    let query = Query::new(
      QueryType::Insert { table: "my_table", values: None }, 
      vec!["my_table.name", "my_table.value"]
    ).to_string();
    let expected = String::from(
      "INSERT INTO my_table (my_table.name,my_table.value) VALUES (?,?) "
    );
    assert_eq!(query, expected)
  }

}