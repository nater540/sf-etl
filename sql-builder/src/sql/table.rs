use std::collections::HashMap;

use super::{
  SqlGenerator,
  types::Type
};

#[derive(Debug, Clone)]
pub struct Table {
  name: String,
  columns: HashMap<String, Type>
}

impl Table {
  pub fn new<N>(name: N) -> Self
  where N: Into<String> {
    Table {
      name:    name.into(),
      columns: HashMap::new()
    }
  }

  pub fn name(&self) -> String {
    self.name.clone()
  }

  pub fn add_column<N>(&mut self, name: N, tp: Type) -> &mut Self
  where N: Into<String> {
    self.columns.insert(name.into(), tp);
    self
  }

  pub fn generate<T>(&mut self) -> String
  where T: SqlGenerator {

    let (prefix, affix) = T::create_table(&self.name);
    let col_count    = self.columns.len();

    let mut sql = self.columns
      .iter_mut()
      .enumerate()
      .fold(prefix, |mut sql, (idx, (ref name, ref col_type))| {
        sql.push_str(&T::create_column(name, &col_type));

        if idx < col_count - 1 {
          sql.push_str(",\n");
        }
        sql
      });

      sql.push_str(&affix);
      sql
  }
}
