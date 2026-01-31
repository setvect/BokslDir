#![allow(dead_code)]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BokslDirError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, BokslDirError>;
