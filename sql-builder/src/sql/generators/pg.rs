use crate::sql::{
  types::{BaseType, Type},
  SqlGenerator
};

pub struct Pg;
impl SqlGenerator for Pg {
  fn create_table(name: &str) -> (String, String) {
    (
      format!("CREATE TABLE \"{}\" (\n", name), // Prefix
      "\n)".to_owned()                          // Affix
    )
  }

  fn create_column(name: &str, tp: &Type) -> String {
    use self::BaseType::*;

    // Get the column type
    let inner = tp.inner();

    format!(
      "{}{}{}{}{}",
      match inner {
        Foreign(_, _) => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Custom(_)     => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Array(it)     => format!("\"{}\" {}", name, Pg::stringify(Array(Box::new(*it)))),
        Varchar(_)    => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Boolean       => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Integer       => format!("\"{}\" {}", name, Pg::stringify(inner)),
        BigInt        => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Text          => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Float         => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Double        => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Jsonb         => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Date          => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Time          => format!("\"{}\" {}", name, Pg::stringify(inner)),
        DateTime      => format!("\"{}\" {}", name, Pg::stringify(inner)),
        Index(_)      => panic!("`create_column` should not be called for indices")
      },
      match tp.primary {
        true  => " PRIMARY KEY",
        false => ""
      },
      match (&tp.default).as_ref() {
        Some(ref default) => format!(" DEFAULT '{}'", default),
        _                 => format!("")
      },
      match tp.nullable {
        false => " NOT NULL",
        true  => ""
      },
      match tp.unique {
        true  => " UNIQUE",
        false => ""
      }
    )
  }
}

impl Pg {
  fn stringify(tp: BaseType) -> String {
    use self::BaseType::*;

    match tp {
      Foreign(tbl, refs) => format!("VARCHAR REFERENCES \"{}\" ({})", tbl, refs.0.join(",")),
      Custom(sql)        => format!("{}", sql),
      Array(boxed)       => format!("{}[]", Pg::stringify(*boxed)),
      Varchar(Some(len)) => match len {
        0 => format!("VARCHAR"),
        _ => format!("VARCHAR({})", len)
      },
      Varchar(None)      => format!("VARCHAR"),
      Boolean            => format!("BOOLEAN"),
      Integer            => format!("INTEGER"),
      BigInt             => format!("BIGINT"),
      Text               => format!("TEXT"),
      Float              => format!("FLOAT"),
      Double             => format!("DOUBLE PRECISION"),
      Jsonb              => format!("JSONB"),
      Time               => format!("TIME"),
      Date               => format!("DATE"),
      DateTime           => format!("TIMESTAMP"),
      _                  => unreachable!()
    }
  }
}
