use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorClientError {
    #[error("There was an issue with the REST request - {source}")]
    ReqwestClientError {
        #[from]
        source: reqwest::Error,
    },
    #[error("There was an issue with the validator client - {0}")]
    ValidatorError(String),
}

#[derive(Error, Debug, Deserialize)]
#[error("code: {code} - {message}")]
pub(super) struct CodedError {
    code: u32,
    message: String,
    details: Vec<(String, String)>,
}

#[derive(Error, Deserialize, Debug)]
#[error("{error}")]
pub(super) struct SmartQueryError {
    pub(super) error: String,
}
