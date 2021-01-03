use std::{
  fmt::{self, Display, Formatter},
  time::SystemTime
};

#[derive(PartialEq, Debug, Clone)]
pub struct WrapVec<T>(pub Vec<T>);

impl<'a> From<&'a str> for WrapVec<String> {
  fn from(s: &'a str) -> Self {
    WrapVec(vec![s.into()])
  }
}

impl From<String> for WrapVec<String> {
  fn from(s: String) -> Self {
    WrapVec(vec![s])
  }
}

impl<I> From<Vec<I>> for WrapVec<String>
where I: Into<String> {
  fn from(v: Vec<I>) -> Self {
    WrapVec(v.into_iter().map(|s| s.into()).collect())
  }
}

#[derive(Debug, PartialEq, Clone)]
pub enum BaseType {
  Foreign(String, WrapVec<String>),
  Custom(&'static str),
  Array(Box<BaseType>),
  Index(Vec<String>),
  Varchar(Option<usize>),
  Boolean,
  Integer,
  BigInt,
  Text,
  Float,
  Double,
  Jsonb,
  DateTime,
  Time,
  Date
}

#[derive(PartialEq, Debug, Clone)]
pub enum WrappedDefault<'a> {
  Array(Vec<Type>),
  Text(&'a str),
  Integer(i64),
  BigInt(i128),
  Float(f32),
  Double(f64),
  Boolean(bool),
  Date(SystemTime),
  DateTime(SystemTime),
  Foreign(Box<Type>),
  Custom(&'static str)
}

impl<'outer> Display for WrappedDefault<'outer> {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    use self::WrappedDefault::*;
    write!(f, "{}", &match *self {
      Text(ref val)     => format!("{}", val),
      BigInt(ref val)   => format!("{}", val),
      Integer(ref val)  => format!("{}", val),
      Float(ref val)    => format!("{}", val),
      Double(ref val)   => format!("{}", val),
      Boolean(ref val)  => format!("{}", val),
      Date(ref val)     => format!("{:?}", val),
      DateTime(ref val) => format!("{:?}", val),
      Foreign(ref val)  => format!("{:?}", val),
      Custom(ref val)   => format!("{}", val),
      Array(ref val)    => format!("{:?}", val)
    })
  }
}

impl From<&'static str> for WrappedDefault<'static> {
  fn from(val: &'static str) -> Self {
    WrappedDefault::Text(val)
  }
}

impl From<bool> for WrappedDefault<'static> {
  fn from(val: bool) -> Self {
    WrappedDefault::Boolean(val)
  }
}

impl From<i64> for WrappedDefault<'static> {
  fn from(val: i64) -> Self {
    WrappedDefault::Integer(val)
  }
}

impl From<f32> for WrappedDefault<'static> {
  fn from(val: f32) -> Self {
    WrappedDefault::Float(val)
  }
}

impl From<f64> for WrappedDefault<'static> {
  fn from(val: f64) -> Self {
    WrappedDefault::Double(val)
  }
}

impl From<SystemTime> for WrappedDefault<'static> {
  fn from(time: SystemTime) -> Self {
    WrappedDefault::Date(time)
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Type {
  pub default:    Option<WrappedDefault<'static>>,
  pub size:       Option<usize>,
  pub inner:      BaseType,
  pub nullable:   bool,
  pub unique:     bool,
  pub increments: bool,
  pub indexed:    bool,
  pub primary:    bool
}

impl Default for Type {
  fn default() -> Self {
    Type {
      nullable:   false,
      unique:     false,
      increments: false,
      indexed:    false,
      primary:    false,
      default:    None,
      size:       None,
      inner:      BaseType::Integer
    }
  }
}

impl Type {
  pub(crate) fn new(inner: BaseType) -> Self {
    Self { inner, ..Default::default() }
  }

  pub fn nullable(self, val: bool) -> Self {
    Self { nullable: val, ..self }
  }

  pub fn unique(self, val: bool) -> Self {
    Self { unique: val, ..self }
  }

  pub fn increments(self, val: bool) -> Self {
    Self { increments: val, ..self }
  }

  pub fn indexed(self, val: bool) -> Self {
    Self { indexed: val, ..self }
  }

  pub fn primary(self, val: bool) -> Self {
    Self { primary: val, ..self }
  }

  pub fn size(self, val: usize) -> Self {
    Self { size: Some(val), ..self }
  }

  pub fn default(self, arg: impl Into<WrappedDefault<'static>>) -> Self {
    Self { default: Some(arg.into()), ..self }
  }

  pub(crate) fn inner(&self) -> BaseType {
    self.inner.clone()
  }
}

pub fn integer() -> Type {
  Type::new(BaseType::Integer)
}

pub fn bigint() -> Type {
  Type::new(BaseType::BigInt)
}

pub fn float() -> Type {
  Type::new(BaseType::Float)
}

pub fn double() -> Type {
  Type::new(BaseType::Double)
}

pub fn boolean() -> Type {
  Type::new(BaseType::Boolean)
}

pub fn varchar(len: Option<usize>) -> Type {
  Type::new(BaseType::Varchar(len))
}

pub fn text() -> Type {
  Type::new(BaseType::Text)
}

pub fn time() -> Type {
  Type::new(BaseType::Time)
}

pub fn date() -> Type {
  Type::new(BaseType::Date)
}

pub fn datetime() -> Type {
  Type::new(BaseType::DateTime)
}

pub fn jsonb() -> Type {
  Type::new(BaseType::Jsonb)
}

pub fn custom(sql: &'static str) -> Type {
  Type::new(BaseType::Custom(sql))
}

pub fn foreign<T, K>(table: T, keys: K) -> Type
where T: Into<String>, K: Into<WrapVec<String>> {
  Type::new(BaseType::Foreign(table.into(), keys.into()))
}

pub fn array(inner: &Type) -> Type {
  Type::new(BaseType::Array(Box::new(inner.inner())))
}

pub fn index<S>(columns: Vec<S>) -> Type
where S: Into<String> {
  let vec: Vec<String> = columns.into_iter().map(|s| s.into()).collect();
  Type::new(BaseType::Index(vec))
}
