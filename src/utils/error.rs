#![allow(dead_code)]

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BokslDirError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Not a directory: {path}")]
    NotADirectory { path: PathBuf },

    // Phase 3.2: 파일 복사/이동 에러
    #[error("File already exists: {path}")]
    FileExists { path: PathBuf },

    #[error("Source and destination are the same: {path}")]
    SameSourceAndDest { path: PathBuf },

    #[error("Copy failed: {src} -> {dest}: {reason}")]
    CopyFailed {
        src: PathBuf,
        dest: PathBuf,
        reason: String,
    },

    #[error("Move failed: {src} -> {dest}: {reason}")]
    MoveFailed {
        src: PathBuf,
        dest: PathBuf,
        reason: String,
    },

    #[error("Delete failed: {path}: {reason}")]
    DeleteFailed { path: PathBuf, reason: String },

    #[error("Rename failed: {src} -> {dest}: {reason}")]
    RenameFailed {
        src: PathBuf,
        dest: PathBuf,
        reason: String,
    },

    #[error("Operation cancelled")]
    OperationCancelled,
}

pub type Result<T> = std::result::Result<T, BokslDirError>;
