mod generators;
mod table;
mod types;

pub use generators::*;
pub use table::*;
pub use types::*;

pub trait SqlGenerator {
  fn create_table(name: &str) -> (String, String);
  fn create_column(name: &str, tp: &Type) -> String;
}
