#![allow(dead_code)]

use std::borrow::Cow;

use reqwest::header::{HeaderMap, AUTHORIZATION, ACCEPT};
use serde::{de::DeserializeOwned, Serialize};

use crate::response::*;
use crate::errors::*;

#[derive(Debug, Clone)]
pub struct AccessToken {
  pub token_type: String,
  pub value:      String,
  pub issued_at:  String
}

impl From<TokenResponse> for AccessToken {
  fn from(res: TokenResponse) -> Self {
    AccessToken {
      token_type: res.token_type,
      issued_at:  res.issued_at,
      value:      res.access_token
    }
  }
}

#[derive(Debug)]
pub struct Client {
  http_client:    reqwest::Client,
  client_id:      String,
  client_secret:  String,
  login_endpoint: String,
  version:        String,
  base_path:      Option<String>,
  instance_url:   Option<String>,
  access_token:   Option<AccessToken>
}

/// It builds clients - fairly self explanatory I'd hope.
#[derive(Debug)]
pub struct ClientBuilder<'a> {
  client_id:      Option<Cow<'a, str>>,
  client_secret:  Option<Cow<'a, str>>,
  login_endpoint: Option<Cow<'a, str>>,
  instance_url:   Option<Cow<'a, str>>,
  version:        Option<Cow<'a, str>>
}

impl<'a> Default for ClientBuilder<'a> {
  fn default() -> ClientBuilder<'a> {
    ClientBuilder {
      client_id:      None,
      client_secret:  None,
      instance_url:   None,
      version:        Some(Cow::Borrowed("v49.0")),
      login_endpoint: Some(Cow::Borrowed("https://login.salesforce.com"))
    }
  }
}

impl<'a> ClientBuilder<'a> {
  #[inline]
  pub fn client_id<S>(&mut self, client_id: S) -> &mut Self
  where S: Into<Cow<'a, str>> {
    self.client_id = Some(client_id.into());
    self
  }

  #[inline]
  pub fn client_secret<S>(&mut self, client_secret: S) -> &mut Self
  where S: Into<Cow<'a, str>> {
    self.client_secret = Some(client_secret.into());
    self
  }

  #[inline]
  pub fn login_endpoint<S>(&mut self, login_endpoint: S) -> &mut Self
  where S: Into<Cow<'a, str>> {
    self.login_endpoint = Some(login_endpoint.into());
    self
  }

  #[inline]
  pub fn instance_url<S>(&mut self, instance_url: S) -> &mut Self
  where S: Into<Cow<'a, str>> {
    self.instance_url = Some(instance_url.into());
    self
  }

  #[inline]
  pub fn version<S>(&mut self, version: S) -> &mut Self
  where S: Into<Cow<'a, str>> {
    self.version = Some(version.into());
    self
  }

  /// Consumes the builder & creates a new client.
  pub fn create(&self) -> Result<Client> {
    let client_id = match self.client_id {
      Some(ref cid) => cid.to_owned().to_string(),
      None          => return Err(Error::ClientBuilderError("Must specify `client_id`".to_string()))
    };

    let client_secret = match self.client_secret {
      Some(ref cs) => cs.to_owned().to_string(),
      None         => return Err(Error::ClientBuilderError("Must specify `client_secret`".to_string()))
    };

    let login_endpoint = match self.login_endpoint {
      Some(ref ep) => ep.to_owned().to_string(),
      None         => return Err(Error::ClientBuilderError("Must specify `login_endpoint`".to_string()))
    };

    let version = match self.version {
      Some(ref vers) => vers.to_owned().to_string(),
      None           => return Err(Error::ClientBuilderError("Must specify `version` (This should never happen...)".to_string()))
    };

    let instance_url = match self.instance_url {
      Some(ref ep) => Some(ep.to_owned().to_string()),
      None         => None
    };

    Ok(Client {
      http_client:    reqwest::Client::new(),
      client_id:      client_id,
      client_secret:  client_secret,
      login_endpoint: login_endpoint,
      instance_url:   instance_url,
      access_token:   None,
      base_path:      None,
      version:        version
    })
  }
}

impl Client {
  pub fn builder<'a>() -> ClientBuilder<'a> {
    ClientBuilder::default()
  }

  /// Attempt to login to the Salesforce REST API using the `password` grant type.
  pub async fn login_with_credentials<U, P>(&mut self, username: U, password: P) -> Result<()>
  where U: Into<String>, P: Into<String> {
    // https://developer.salesforce.com/docs/atlas.en-us.api_iot.meta/api_iot/qs_auth_access_token.htm
    let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
    let params = [
      ("grant_type",    "password"),
      ("client_id",     self.client_id.as_str()),
      ("client_secret", self.client_secret.as_str()),
      ("username",      &username.into()),
      ("password",      &password.into()),
    ];

    let res = self
      .http_client
      .post(token_url.as_str())
      .form(&params)
      .send()
      .await?;

    if res.status().is_success() {
      let res: TokenResponse = res.json().await?;

      self.access_token = Some(AccessToken {
        token_type: res.token_type,
        issued_at:  res.issued_at,
        value:      res.access_token
      });

      self.instance_url = Some(res.instance_url);

      // Build a string representing the base path for all further requests
      self.base_path = Some(
        format!("{}/services/data/{}",
        self.instance_url.as_ref().unwrap(),  // Safe to unwrap here since we know this field exists at this point
        self.version
      ));

      // Great success!
      Ok(())
    } else {
      // Uh-Oh Spaghettios!
      let token_error = res.json().await?;
      Err(Error::TokenError(token_error))
    }
  }

  /// Clones the access token (if one exists)
  pub fn access_token(&self) -> Result<AccessToken> {
    match self.access_token.as_ref() {
      Some(token) => Ok(token.clone()),
      None        => Err(Error::NotAuthenticatedError)
    }
  }

  /// Perform an SOQL query.
  pub async fn query<'a, Q, T: DeserializeOwned>(&self, query: Q) -> Result<QueryResponse<T>>
  where Q: Into<&'a str> {
    let url    = format!("{}/query", self.base_path()?);
    let params = vec![("q", query.into())];

    Ok(self.get(&url, Some(params)).await?)
  }

  /// Describe an SObject resource.
  pub async fn describe<'a, N>(&self, name: N) -> Result<DescribeResponse>
  where N: Into<&'a str> {
    let url = format!("{}/sobjects/{}/describe", self.base_path()?, name.into());
    Ok(self.get(&url, None).await?)
  }

  /// Create a bulk query job.
  pub async fn create_query_job<'a, N, F>(&self, from: N, fields: F) -> Result<BulkQueryStatusResponse>
  where N: Into<&'a str>, F: Into<Vec<&'a str>> {
    let query = format!("SELECT {} FROM {}", fields.into().join(","), from.into());

    let params = [
      ("operation", "query"),
      ("query", query.as_str())
    ];

    let url = format!("{}/jobs/query", self.base_path()?);
    Ok(self.post(&url, params).await?)
  }

  /// Get the status of a previously created bulk query job.
  pub async fn get_query_job_status<'a, N>(&self, job_id: N) -> Result<BulkQueryStatusResponse>
  where N: Into<&'a str> {
    let url = format!("{}/jobs/query/{}", self.base_path()?, job_id.into());
    Ok(self.get(&url, None).await?)
  }

  /// Attempt to abort a previously created bulk query job.
  /// You can only abort jobs that are in the following states:
  ///   - UploadComplete
  ///   - InProgress
  pub async fn abort_query_job<'a, N>(&self, job_id: N) -> Result<BulkQueryStatusResponse>
  where N: Into<&'a str> {
    let url = format!("{}/jobs/query/{}", self.base_path()?, job_id.into());
    Ok(self.patch(&url, [("state", "Aborted")]).await?)
  }

  /// Helper function to perform a GET request with JSON deserialization.
  async fn get<T: DeserializeOwned>(&self, url: &str, params: Option<Vec<(&str, &str)>>) -> Result<T> {
    let res = self
      .http_client
      .get(url)
      .headers(self.default_headers()?)
      .query(&params)
      .send()
      .await?;

    if res.status().is_success() {
      Ok(res.json::<T>().await?)
    } else {
      let error = res.json().await?;
      Err(Error::ResponseError(error))
    }
  }

  /// Helper function to perform a POST request with a JSON payload.
  async fn post<T, P>(&self, url: &str, params: P) -> Result<T>
  where T: DeserializeOwned, P: Serialize {
    let res = self
    .http_client
    .post(url)
    .headers(self.default_headers()?)
    .json(&params)
    .send()
    .await?;

    if res.status().is_success() {
      Ok(res.json::<T>().await?)
    } else {
      let error = res.json().await?;
      Err(Error::ResponseError(error))
    }
  }

  /// Helper function to perform a POST request with a JSON payload, but without response deserialization.
  async fn raw_post<P: Serialize>(&self, url: &str, params: P) -> Result<reqwest::Response> {
    Ok(
      self
        .http_client
        .post(url)
        .headers(self.default_headers()?)
        .json(&params)
        .send()
        .await?
    )
  }

  /// Helper function to perform a PATCH request with a JSON payload.
  async fn patch<T, P>(&self, url: &str, params: P) -> Result<T>
  where T: DeserializeOwned, P: Serialize {
    let res = self
    .http_client
    .patch(url)
    .headers(self.default_headers()?)
    .json(&params)
    .send()
    .await?;

    if res.status().is_success() {
      Ok(res.json::<T>().await?)
    } else {
      let error = res.json().await?;
      Err(Error::ResponseError(error))
    }
  }

  /// Builds a set of default headers for all authenticated requests.
  fn default_headers(&self) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, format!("Bearer {}", self.access_token.as_ref().ok_or(Error::NotAuthenticatedError)?.value).parse()?);
    headers.insert(ACCEPT, "application/json".parse()?);

    Ok(headers)
  }

  /// I got tired of typing this over and over; helper function seemed like the next logical step.
  fn base_path(&self) -> Result<&str> {
    Ok(self.base_path.as_ref().ok_or(Error::NotAuthenticatedError)?)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::errors::Result;

  use serde::{Deserialize, Serialize};
  use serde_json::json;
  use mockito::*;

  #[derive(Deserialize, Serialize)]
  #[serde(rename_all = "PascalCase")]
  struct Case {
    id:          String,
    account_id:  String,
    contact_id:  String,
    description: String
  }

  #[tokio::test]
  async fn login_with_credentials() -> Result<()> {
    let mock = build_mock_server("POST", "/services/oauth2/token", mock_token_response(), 200).expect_at_most(1);

    let mut client = Client::builder()
      .client_id("top_secret_thingy")
      .client_secret("even_more_top_secret_thingy")
      .login_endpoint(&mockito::server_url())
      .create()?;

    client.login_with_credentials(
      "supreme.leader@shibe.com".to_string(),
      "this.is.my.password.there.are.many.others.like.it.but.this.one.is.mine".to_string()
    ).await?;

    let token = client.access_token()?;
    assert_eq!("00DR00000008oBT!AQwAQCPqzc_HBE59c80QmEJD4rQKRRc1GRLvYZEq", token.value);
    assert_eq!("1513887500425", token.issued_at);
    assert_eq!("Bearer", token.token_type);
    mock.assert();
    Ok(())
  }

  #[tokio::test]
  async fn query() -> Result<()> {
    // let _ = env_logger::try_init();

    let mock   = build_mock_server("GET", "/services/data/v49.0/query?q=SELECT+Id%2C+AccountId%2C+ContactId%2C+Description+FROM+Case", mock_query_response(), 200).expect_at_most(1);
    let client = build_test_client();
    let res: QueryResponse<Case> = client.query("SELECT Id, AccountId, ContactId, Description FROM Case").await?;

    assert_eq!(res.done, true);
    assert_eq!(res.total_size, 1);
    assert_eq!(res.records[0].id, "0122T000000gkLXQAY");
    assert_eq!(res.records[0].description, "Halp! Everything is on fire!!");
    mock.assert();
    Ok(())
  }

  #[tokio::test]
  async fn describe() -> Result<()> {
    let mock   = build_mock_server("GET", "/services/data/v49.0/sobjects/Case/describe", mock_describe_response(), 200).expect_at_most(1);
    let client = build_test_client();
    let res    = client.describe("Case").await?;

    assert_eq!(res.name, "Case");
    assert_eq!(res.fields.len(), 2);
    assert_eq!(res.fields[0].name, "Id");
    mock.assert();
    Ok(())
  }

  #[tokio::test]
  async fn create_query_job() -> Result<()> {
    let mock   = build_mock_server("POST", "/services/data/v49.0/jobs/query", mock_job_response(), 200).expect_at_most(1);
    let client = build_test_client();
    let res    = client.create_query_job("Account", vec!["Id", "AccountNumber", "Description"]).await?;

    assert_eq!(res.object, "Account");
    assert_eq!(res.id, "750R0000000zlh9IAA");
    mock.assert();
    Ok(())
  }

  /// Does exactly what it says it does...
  fn build_test_client() -> Client {
    let api_version = "v49.0".to_string();
    let base_path   = format!("{}/services/data/{}", &mockito::server_url(), api_version);

    Client {
      http_client:    reqwest::Client::new(),
      client_id:      "top-secret".to_string(),
      client_secret:  "even-more-top-secret".to_string(),
      login_endpoint: "https://example.com".to_string(),
      version:        api_version,
      base_path:      Some(base_path),
      instance_url:   Some(mockito::server_url()),
      access_token:   Some(AccessToken { value: "shiba".to_string(), token_type: "Bearer".to_string(), issued_at: "1513887500425".to_string() })
    }
  }

  /// This also does exactly what it says it does...
  fn build_mock_server<P, B>(method: &str, path: P, body: B, status: usize) -> Mock
  where P: Into<Matcher>, B: AsRef<[u8]> {
    mock(method, path)
      .with_status(status)
      .with_header("content-type", "application/json")
      .with_body(body)
      .create()
  }

  /*
   * Just a bunch of mock JSON blobs below; nothing really existing, trust me.
  */

  fn mock_token_response() -> String {
    json!({
      "access_token": "00DR00000008oBT!AQwAQCPqzc_HBE59c80QmEJD4rQKRRc1GRLvYZEq",
      "instance_url": "https://MyDomainName.my.salesforce.com",
      "id": "https://login.salesforce.com/id/00DR00000008oBTMAY/005R0000000IUUMIA4",
      "token_type": "Bearer",
      "issued_at": "1513887500425",
      "signature": "3PiFUIioqKkHpHxUiCCDzpvSiM2F6//w2/CslNTuf+o="
    }).to_string()
  }

  fn mock_query_response() -> String {
    json!({
      "totalSize": 1,
      "done": true,
      "records": vec![
        Case {
          id:          "0122T000000gkLXQAY".to_string(),
          account_id:  "01234000000BnaHAAS".to_string(),
          contact_id:  "01280000000HgqbAAC".to_string(),
          description: "Halp! Everything is on fire!!".to_string()
        }
      ]
    }).to_string()
  }

  fn mock_job_response() -> String {
    json!({
      "id":              "750R0000000zlh9IAA",
      "operation":       "query",
      "object":          "Account",
      "createdById":     "005R0000000GiwjIAC",
      "createdDate":     "2018-12-10T17:50:19.000+0000",
      "systemModstamp":  "2018-12-10T17:50:19.000+0000",
      "state":           "InProgress",
      "concurrencyMode": "Parallel",
      "contentType":     "CSV",
      "apiVersion":      46.0,
      "lineEnding":      "LF",
      "columnDelimiter": "COMMA"
    }).to_string()
  }

  fn mock_describe_response() -> String {
    use crate::response::*;

    json!({
      "name": "Case",
      "fields": vec![
        Field { name: "Id".to_string(),        length: 42, custom: false, encrypted: false, precision: 0, updateable: false, nillable: false, unique: true,  relationship_name: None, field_type: FieldType::Id },
        Field { name: "AccountId".to_string(), length: 42, custom: false, encrypted: false, precision: 0, updateable: false, nillable: false, unique: false, relationship_name: None, field_type: FieldType::Id }
      ]
    }).to_string()
  }
}
