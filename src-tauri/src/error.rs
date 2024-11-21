use serde::Serialize;
use thiserror::Error;

// error object
#[derive(Error, Debug, PartialEq, Serialize)]
pub enum Error {
    #[error("unable to retrieve file: {0:?}")]
    IO(String),
    #[error("usb connection failed: {0:?}")]
    Usb(String),
    #[error("failed to make a request: {0:?}")]
    Http(String),
    #[error("other error: {0:?}")]
    Other(String),
    #[error("serial connection failed: {0:?}")]
    Serial(String),
    #[error("failed to complete install: {0:?}")]
    Install(String),
    #[error("failed to enter bootloader: {0:?}")]
    Bootloader(String),
    #[error("incompatable version: {0:?}")]
    Incompatable(String),
}

pub type Result<T> = ::std::result::Result<T, Error>;
