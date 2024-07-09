use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("EOF")]
    EOF,
    #[error("intercepted")]
    Intercepted,
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),
}