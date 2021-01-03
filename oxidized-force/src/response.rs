use serde::{
  //de::{Deserializer, DeserializeOwned},
  Deserialize
};

/// Represents a successful query response.
#[derive(Deserialize, Debug, Clone)]
pub struct QueryResponse<T> {
  pub total_size: i32,
  pub done:       bool,
  pub records:    Vec<T>
}

/// Represents a successful token request response.
#[derive(Deserialize, Debug, Clone)]
pub struct TokenResponse {
  pub id:           String,
  pub issued_at:    String,
  pub access_token: String,
  pub instance_url: String,
  pub signature:    String,
  pub token_type:   Option<String>
}

/// Represents a failed token request response.
#[derive(Deserialize, Debug, Clone)]
pub struct TokenErrorResponse {
  error_description: String,
  error: String
}

/// Represents the response from creating query jobs & fetching their statuses.
#[serde(rename_all = "camelCase")]
#[derive(Deserialize, Debug, Clone)]
pub struct BulkQueryStatusResponse {
  pub id:               String,
  pub operation:        String,
  pub object:           String,
  pub created_date:     String,
  pub state:            BulkState,
  pub concurrency_mode: String,
  pub content_type:     String,
  pub api_version:      String,
  pub line_ending:      String,
  pub column_delimiter: String
}

/// Represents the possible bulk query states.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum BulkState {
  UploadComplete,
  InProgress,
  Aborted,
  JobComplete,
  Failed
}

/// Represents a successful describe request response.
/// See https://developer.salesforce.com/docs/atlas.en-us.uiapi.meta/uiapi/ui_api_responses_object_info.htm
#[serde(rename_all = "camelCase")]
#[derive(Deserialize, Debug, Clone)]
pub struct DescribeResponse {
  pub name:   String,
  pub fields: Vec<Field>
}

#[serde(rename_all = "camelCase")]
#[derive(Deserialize, Debug, Clone)]
pub struct Field {
  pub name:       String,
  pub length:     i32,
  pub custom:     bool,
  pub encrypted:  bool,
  pub precision:  u8,
  pub updateable: bool,
  pub nillable:   bool,
  pub unique:     bool,

  pub relationship_name: Option<String>,

  // TODO: Make this work...
  //#[serde(skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_default_value")]
  //pub default_value: Option<String>,

  #[serde(rename = "type")]
  pub field_type: FieldType
}

/// Represents a generic error response.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
  pub message: String,
  pub error_code: String,
  pub fields: Option<Vec<String>>
}

/// Represents all of the possible field types contained inside various responses.
/// See https://developer.salesforce.com/docs/atlas.en-us.object_reference.meta/object_reference/primitive_data_types.htm
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FieldType {
  Id,
  Base64,
  Boolean,
  Byte,
  Date,
  Double,
  Int,
  Long,
  String,
  Time,
  Address,
  AnyType,
  Calculated,
  Currency,
  Email,
  JunctionIdList,
  Location,
  Percent,
  Phone,
  Picklist,
  Reference,
  Url,

  // Renamed field types below this point

  #[serde(rename = "textarea")]
  TextArea,

  #[serde(rename = "datetime")]
  DateTime,

  #[serde(rename = "combobox")]
  ComboBox,

  #[serde(rename = "encryptedstring")]
  EncryptedString,

  #[serde(rename = "masterrecord")]
  MasterRecord,

  #[serde(rename = "multipicklist")]
  MultiPicklist
}

impl DescribeResponse {
  /// Gets a vector of all field names (useful for bulk queries)
  pub fn field_names(&self) -> Vec<String> {
    self
      .fields
      .iter()
      .map(|f| f.name.clone())
      .collect()
  }
}

// fn deserialize_default_value<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
// where D: Deserializer<'de> {
//   #[derive(Deserialize)]
//   struct DefaultValue {
//     value: String
//   }

//   DefaultValue::deserialize(deserializer).map(|d| Some(d.value))
// }
