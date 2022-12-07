use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, ValmanError>;

#[derive(Debug, ThisError)]
pub enum ValmanError {
    #[error("Docker API error - {0}")]
    DockerApi(#[from] docker_api::Error),

    #[error("Docker error - {0}")]
    Docker(String),

    #[error("Valve A2S error - {0}")]
    ValveA2S(#[from] a2s::errors::Error),

    #[error("Backup error - {0}")]
    Backup(#[from] std::io::Error),
}
