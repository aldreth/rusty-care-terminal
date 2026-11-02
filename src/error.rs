use thiserror::Error;

#[derive(Debug, Error)]
pub enum CareError {
    #[error("config error: {0}")]
    Config(String),

    #[error("git error in {path}: {message}")]
    Git { path: String, message: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CareError>;
