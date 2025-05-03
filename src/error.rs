use crate::Error::ErrorWrapper;
use std::fmt::Display;
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::Arc;
use thiserror::Error;

pub type Result<E> = std::result::Result<E, Error>;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("General Error")]
    GeneralError(String),
    #[error("Error Wrapper {}", .0)]
    ErrorWrapper(String, Arc<dyn std::error::Error + Send + Sync + 'static>),
    #[error("Failed to convert bytes to string: {0}")]
    Utf8ConversionError(#[from] Utf8Error),
    #[error("Failed to convert bytes to string: {0}")]
    FromUtf8ConversionError(#[from] FromUtf8Error),
    #[error("Parser error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("Cant handle request")]
    CantHandle,
}

pub trait Context<T, E> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;
    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T, E> Context<T, E> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|e| ErrorWrapper(context.to_string(), Arc::new(e)))
    }

    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| ErrorWrapper(context().to_string(), Arc::new(e)))
    }
}
