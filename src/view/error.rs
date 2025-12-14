use thiserror::Error;

#[derive(Error, Debug)]
pub enum ViewError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ViewError>;
