#![allow(unused_imports)]
#![allow(dead_code)]

#[macro_use]
extern crate log;

use std::path::PathBuf;
use std::io::Write;
use std::fs::File;

use structopt::StructOpt;

use oxidized_force::prelude::*;

mod sql;
use sql::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "sf-sql", about = "Builds SQL for Salesforce objects")]
struct Opts {
  #[structopt(long, short = "c")]
  client_id: String,

  /// Keep it secret, keep it safe
  #[structopt(long, short = "s")]
  client_secret: String,

  /// Salesforce login endpoint
  #[structopt(long, short = "e", default_value = "https://login.salesforce.com")]
  login_endpoint: String,

  #[structopt(long, short = "u")]
  username: String,

  #[structopt(long, short = "p")]
  password: String,

  /// Salesforce SObject name
  #[structopt(long, short)]
  name: String,

  /// Output file path
  #[structopt(long, short)]
  output: PathBuf
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  pretty_env_logger::init();
  let args = Opts::from_args();

  let mut client = Client::builder()
    .client_id(args.client_id)
    .client_secret(args.client_secret)
    .login_endpoint(args.login_endpoint)
    .create()?;

  info!("Attempting to log into Salesforce...");
  client.login_with_credentials(args.username, args.password).await?;

  info!("Describing object...");
  let desc = client.describe(args.name.as_ref()).await?;
  let mut table = Table::new(args.name);

  // Create columns for all of the object fields
  for field in &desc.fields {
    let column = column_from_field(&field)
      .nullable(field.nillable)
      .unique(field.unique);

    table.add_column(&field.name, column);
  }

  info!("Writing SQL file...");
  let mut output = File::create(args.output)?;
  output.write_all(table.generate::<Pg>().as_bytes())?;

  Ok(())
}

fn column_from_field(field: &oxidized_force::response::Field) -> Type {
  use oxidized_force::response::FieldType::*;

  match &field.field_type {
    MultiPicklist => array(&varchar(None)),
    Reference     => foreign(field.relationship_name.as_ref().unwrap(), vec!["Id"]),
    Id            => varchar(None).primary(true),
    AnyType       => jsonb(),
    Boolean       => boolean(),
    Time          => time(),
    Date          => date(),
    DateTime      => datetime(),
    Double        => double(),
    Int           => integer(),
    Long          => bigint(),
    _             => varchar(Some(field.length as usize))
  }
}
