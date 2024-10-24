use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("LWT promise was rejected with exception: {0}")]
    LwtPromiseRejection(String),
}
