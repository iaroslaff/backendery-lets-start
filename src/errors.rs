use thiserror::Error;

#[derive(Debug, Error)]
pub(super) enum SecretError {
    #[error("the `{0}` secret was not found")]
    MissingSecret(&'static str),

    #[allow(dead_code)]
    #[error("can't convert `{0}` to the expected type")]
    InvalidSecret(&'static str),
}
