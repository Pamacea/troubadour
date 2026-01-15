//! Audio device abstractions

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Stream error: {0}")]
    StreamError(String),
}

pub type Result<T> = std::result::Result<T, AudioError>;
