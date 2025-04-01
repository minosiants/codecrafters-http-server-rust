use nom::error::ParseError;
use thiserror::Error;
use nom::Err as NomErr;
pub type Result<E> = std::result::Result<E, Error>;


#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("General Error")]
    GeneralError(String)

}