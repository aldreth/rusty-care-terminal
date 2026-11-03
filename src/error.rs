use thiserror::Error;

use crate::services::http::HttpError;

#[derive(Debug, Error)]
pub enum CareError {
    #[error("config error: {0}")]
    Config(String),

    #[error("git error in {path}: {message}")]
    Git { path: String, message: String },

    #[error("care feed error: {0}")]
    Feed(String),

    #[error("weather: {0}")]
    Weather(String),

    #[error(transparent)]
    Http(#[from] HttpError),
}

impl CareError {
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, CareError::Http(HttpError::RateLimited { .. }))
    }
}

pub type Result<T> = std::result::Result<T, CareError>;
