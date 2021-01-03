use crate::response::{ErrorResponse, TokenErrorResponse};

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error("failed to deserialize response")]
  DeserializeError(#[from] serde_json::Error),

  #[error("failed to build force client ({0})")]
  ClientBuilderError(String),

  #[error("must login first")]
  NotAuthenticatedError,

  #[error("token request failed")]
  TokenError(TokenErrorResponse),

  #[error("request failed ({})", .0.message)]
  ResponseError(ErrorResponse),

  #[error("request failed")]
  HttpError(#[from] reqwest::Error),

  #[error("invalid request header")]
  InvalidRequestHeader(#[from] reqwest::header::InvalidHeaderValue)
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
