#![allow(dead_code)]

use std::borrow::Cow;

use reqwest::header::{HeaderMap, AUTHORIZATION, ACCEPT};
use serde::{de::DeserializeOwned, Serialize};

use crate::response::*;
use crate::errors::*;

#[derive(Debug)]
pub struct AccessToken {
  pub token_type: String,
  pub value:      String,
  pub issued_at:  String
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

      // Keep it secret, keep it safe...
      self.access_token = Some(AccessToken {
        token_type: res.token_type.ok_or(Error::ClientBuilderError("Token request failed.".to_string()))?,
        issued_at:  res.issued_at,
        value:      res.access_token
      });

      self.instance_url = Some(res.instance_url);

      // Build a string representing the base path for any further requests
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
