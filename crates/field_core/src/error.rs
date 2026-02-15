use thiserror::Error;

#[derive(Debug, Error, uniffi::Error)]
pub enum RadrootsAppError {
    #[error("{0}")]
    Msg(String),
}
