/// Error types for Migi agent
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MigiError {
    #[error("observer failed: {0}")]
    Observer(String),

    #[error("learner failed: {0}")]
    Learner(String),

    #[error("intervener failed: {0}")]
    Intervener(String),

    #[error("trust boundary violated: {0}")]
    TrustViolation(String),

    #[error("host communication error: {0}")]
    HostCommunication(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("phase transition rejected: {reason}")]
    PhaseTransitionRejected { reason: String },
}

pub type MigiResult<T> = Result<T, MigiError>;
