use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Path is not absolute")]
    PathNotAbsolute,
    #[error("Invalid path")]
    InvalidPath,
    #[error("File not found")]
    FileNotFound,
    #[error("Is directory")]
    IsDirectory,
    #[error("Is not a directory")]
    NotADirectory,
    #[error("File read out of bounds")]
    FileReadOutOfBounds,
    #[error("{0}")]
    Other(anyhow::Error),
}